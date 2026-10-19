// 本次 UI 修复的静态证据；只读取 main、GUI 和已通过验证的构建报告。
const fs = require('fs'), path = require('path'), crypto = require('crypto');
const project = path.resolve(__dirname, '..');
if(process.argv.length!==3) throw Error('用法：node tools/capture-ui-health-evidence.cjs "本地分析 inspect.js"');
const { at, source } = require(path.resolve(process.argv[2]));
const hash = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
for (const [project, offsets] of [
  ['mhgu-target-health-display', [0x18E50, 0x18F90, 0x18FFC, 0x29870, 0x29A58, 0x55D2B4, 0x65E1D0, 0x5610D0, 0x5C5E74, 0x5C5FF4, 0x56E2D8, 0x56DC28, 0x56DC88]],
]) {
  const root = path.resolve(__dirname, '..'), evidence = path.join(root, 'target/evidence/ui-health');
  fs.mkdirSync(evidence, { recursive: true });
  const content = offsets.map(offset => { const f = at(offset); return '// ' + source + ':' + f.line + '\n' + f.text; }).join('\n');
  fs.writeFileSync(path.join(evidence, 'E-001-native.c'), content);
  const verification = fs.readFileSync(path.join(root, 'target/build-verification.json'));
  fs.writeFileSync(path.join(evidence, 'E-002-build-verification.json'), verification);
  fs.writeFileSync(path.join(evidence, 'manifest.json'), JSON.stringify({
    observed_at: new Date().toISOString(), source_ref: source,
    repro_command: 'node "mhgu-target-health-display/tools/capture-ui-health-evidence.cjs"',
    artifacts: [
      { id: 'E-001', path: 'E-001-native.c', sha256: hash(content) },
      { id: 'E-002', path: 'E-002-build-verification.json', sha256: hash(verification) },
    ],
    dynamic_game_test: false,
  }, null, 2) + '\n');
  console.log('Saved evidence: ' + evidence);
}
