"""执行原版 ARM 指令验证名字、分类、发怒与体型；不启动游戏。"""
from pathlib import Path
import argparse
import json
import math
import struct
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
parser = argparse.ArgumentParser(description="需要 Node.js 和 Python unicorn；输入原版 main NSO 与历史怪物字段证据 JSON")
parser.add_argument("main_nso", type=Path)
parser.add_argument("metadata_evidence", type=Path)
args = parser.parse_args()
from unicorn import Uc, UC_ARCH_ARM, UC_MODE_ARM
from unicorn.arm_const import (
    UC_ARM_REG_C1_C0_2, UC_ARM_REG_FPEXC, UC_ARM_REG_SP,
    UC_ARM_REG_LR, UC_ARM_REG_R0, UC_ARM_REG_R1, UC_ARM_REG_R2, UC_ARM_REG_R3,
)

# 从当前真实 NSO 重新提取机器码，避免只执行报告里抄下来的逻辑。
extract = r"""
const fs=require('fs'),path=require('path');
const nso=require(path.join(process.argv[1],'tools/nso.cjs'));
const bytes=fs.readFileSync(process.argv[2]),h=nso.nsoHeader(bytes),memory=Buffer.alloc(h.dataMemOff+h.dataSize+h.bssSize);
for(const [i,part] of ['text','ro','data'].entries()){
 const off=h[part+'FileOff'],length=h[part+'FileSize'];
 const data=h.flags&(1<<i)?nso.decompressSegment(bytes,off,length,h[part+'Size']):bytes.subarray(off,off+length);
 data.copy(memory,h[part+'MemOff']);
}
const read=(rva,length)=>memory.subarray(rva,rva+length);
const ranges=[[0x50B64,0x80],[0x51904,0x24],[0x67310,0x11C],[0x8172C,0x18],
 [0x55FF98,0x100],[0x1597EA4,137*8],[0x162D9F0,123*16],[0x16325A8,32]];
process.stdout.write(JSON.stringify(ranges.map(([rva,length])=>({rva,hex:read(rva,length).toString('hex')}))));
"""
fixture = json.loads(subprocess.check_output(["node", "-e", extract, str(ROOT), str(args.main_nso.resolve())], text=True))
manifest = json.loads(args.metadata_evidence.read_text(encoding="utf-8"))

BASE, HEAP, STACK, STOP = 0x60000000, 0x10000000, 0x11000000, 0x12000000
uc = Uc(UC_ARCH_ARM, UC_MODE_ARM)
uc.reg_write(UC_ARM_REG_C1_C0_2, 0xF00000)
uc.reg_write(UC_ARM_REG_FPEXC, 0x40000000)
for address, length in [(BASE, 0x1700000), (HEAP, 0x100000), (STACK, 0x100000), (STOP, 0x1000)]:
    uc.mem_map(address, length)
for item in fixture:
    uc.mem_write(BASE + item["rva"], bytes.fromhex(item["hex"]))

def word(address, value):
    uc.mem_write(address, struct.pack("<I", value))

def floating(address, value):
    uc.mem_write(address, struct.pack("<f", value))

def call(rva, *args):
    for register, value in zip([UC_ARM_REG_R0, UC_ARM_REG_R1, UC_ARM_REG_R2, UC_ARM_REG_R3], args):
        uc.reg_write(register, value)
    uc.reg_write(UC_ARM_REG_SP, STACK + 0xFF000)
    uc.reg_write(UC_ARM_REG_LR, STOP)
    uc.emu_start(BASE + rva, STOP, count=100000)
    return uc.reg_read(UC_ARM_REG_R0)

name_cases = 0
class_cases = 0
for item in manifest["monster_index"]:
    assert call(0x50B64, item["id"]) == item["name_index"]
    name_cases += 1
    canonical = call(0x55FF98, 0, item["id"], 0)
    assert canonical == item["notebook_id"]
    match = next((row for row in manifest["notebook_entries"] if row["id"] == canonical), None)
    if match:
        assert match["class_enum"] == item["class_enum"]
        category = match["class_enum"]
        index = struct.unpack("<H", uc.mem_read(BASE + 0x16325A8 + category * 2, 2))[0]
        table_key = (17, 9) if category == 14 else (3, index)
        assert table_key == (match["class_message_table"], match["class_message_index"])
        class_cases += 1
assert call(0x50B64, 0xFFFF) == 0xFFFFFFFF
assert class_cases == 129

monster, work = HEAP + 0x1000, HEAP + 0x20000
word(monster + 0x1428, work)
for state in range(256):
    uc.mem_write(work + 0x518, bytes([state]))
    assert call(0x8172C, monster) == (1 if state == 1 else 0)

# 受控体型表夹具；执行的是原版 67310、50B64 与 51904，而非 Python 重写判定。
manager, table, records, output = HEAP + 0x40000, HEAP + 0x50000, HEAP + 0x60000, HEAP + 0x70000
word(manager + 0x2584, table)
word(table + 0x64, 1)
word(table + 0x6C, records)
word(records + 4, 3)  # 原版 Rathalos 名字索引。
floating(records + 8, 1500.0)
floating(records + 12, 0.90)
floating(records + 16, 1.15)
floating(records + 20, 1.20)
size_cases = []
for percent, crown in [(85, 3), (90, 3), (91, 255), (114, 255), (115, 2), (119, 2), (120, 1), (130, 1)]:
    uc.mem_write(output, struct.pack("<B3xf", 255, 0.0))
    call(0x67310, manager, 2, percent, output)
    actual, size = struct.unpack("<B3xf", uc.mem_read(output, 8))
    assert actual == crown, (percent, actual, crown)
    assert math.isclose(size, percent / 100.0 * 1500.0, abs_tol=0.001)
    size_cases.append({"percent": percent, "crown": actual, "size_cm": size})
for label, base_size, percent, monster_id in [
    ("zero percentage", 1500.0, 0, 2),
    ("unsupported base size", -1.0, 100, 2),
    ("record missing", 1500.0, 100, 1),
]:
    floating(records + 8, base_size)
    uc.mem_write(output, struct.pack("<B3xf", 255, 0.0))
    call(0x67310, manager, monster_id, percent, output)
    assert struct.unpack("<B3xf", uc.mem_read(output, 8)) == (255, 0.0), label

result = {
    "target_sha256": manifest["target_sha256"],
    "execution": "original ARM32 machine code under Unicorn with controlled object/table fixtures",
    "name_cases": name_cases, "class_cases": class_cases, "rage_cases": 256,
    "size_cases": size_cases, "unsupported_size_cases": 3,
    "game_runtime_test": False,
}
(ROOT / "target").mkdir(exist_ok=True)
(ROOT / "target/metadata-native-verification.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
print(f"Original ARM metadata passed: {name_cases} names, {class_cases} classes, 256 rage states, 8 crown boundaries, 3 unsupported-size cases.")
