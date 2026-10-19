// 保存离线静态证据；不会修改游戏二进制或 IDA 数据库。
const fs = require('fs'), path = require('path'), crypto = require('crypto');
const root = path.resolve(__dirname, '..');
if(process.argv.length!==3) throw Error('用法：node tools/capture-evidence.cjs "本地分析 inspect.js"');
const destination = path.join(root, 'target/evidence/native-health');
const { at, read, source } = require(path.resolve(process.argv[2]));
fs.mkdirSync(destination, { recursive: true });
const groups = [
  { id: 'E-001', title: '当前锁定目标与同区域过滤', offsets: [0x18E50, 0x18FFC, 0x29868, 0x29A58], work: 'WI-001' },
  { id: 'E-002', title: '怪物 work 指针、当前及最大血量、可见裁剪范围', offsets: [0x9A764, 0xC04D8, 0x3AED0C, 0x86A24C, 0xBC0874], work: 'WI-002' },
];
for (const group of groups) {
  const content = group.offsets.map(offset => { const f = at(offset); return '// ' + source + ':' + f.line + '\n' + f.text; }).join('\n');
  const name = group.id + '-native.c';
  fs.writeFileSync(path.join(destination, name), content);
  fs.writeFileSync(path.join(destination, group.id + '.md'), `### ${group.id}\n\n- title: ${group.title}\n- observed_at: ${new Date().toISOString()}\n- source_type: file\n- source_ref: evidence/${name}\n- content_hash: ${crypto.createHash('sha256').update(content).digest('hex')}\n- artifact_path: evidence/${name}\n- repro_command: node tools/capture-evidence.cjs\n- raw_excerpt: 见完整原生函数，ARM32，地址按 main RVA 记录。\n- linked_workitem: ${group.work}\n- supersedes: none\n- severity: info\n- status: observed\n- notes: 仅静态验证；游戏与手机动态效果未测试。\n`);
}
const ranges = [[0x9A764, 0x54], [0xC04D8, 0x50], [0x86A24C, 0x38]];
const words = ranges.map(([offset, length]) => ({ offset, words: Array.from({ length: length / 4 }, (_, i) => read(offset + i * 4, 4).readUInt32LE()) }));
fs.writeFileSync(path.join(destination, 'native-words.json'), JSON.stringify(words, null, 2) + '\n');
console.log('Saved selected target, HP and cropped-viewport evidence.');
