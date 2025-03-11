const { execSync } = require('child_process');
const fs = require('fs');

// 读取package.json获取当前版本
const packageJson = JSON.parse(fs.readFileSync('./package.json', 'utf8'));
const version = packageJson.version;
const tagName = `v${version}`;

console.log(`执行发布后操作 - 版本: ${version}`);

try {
  // 提交所有更改
  console.log('提交更改...');
  execSync('git add .', { stdio: 'inherit' });
  execSync(`git commit -m "Release ${tagName}"`, { stdio: 'inherit' });
  
  // 创建标签
  console.log(`创建标签 ${tagName}...`);
  execSync(`git tag -a ${tagName} -m "Release ${tagName}"`, { stdio: 'inherit' });
  
  // 推送到远程仓库
  console.log('推送代码和标签到远程仓库...');
  execSync('git push', { stdio: 'inherit' });
  execSync('git push --tags', { stdio: 'inherit' });
  
  console.log('发布后操作完成!');
} catch (error) {
  console.error('发布后操作失败:', error.message);
  process.exit(1);
} 