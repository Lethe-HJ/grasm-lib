const { execSync } = require("child_process");
const fs = require("fs");

// 读取package.json获取当前版本
const packageJson = JSON.parse(fs.readFileSync("./package.json", "utf8"));
const version = packageJson.version;
const tagName = `v${version}`;

const { runScripts } = require("./run-scripts");
runScripts(/*bash*/ `
  # 提交package.json的更改  
  git add package.json
  git commit -m "Release ${tagName}"

  # 创建标签
  git tag -a ${tagName} -m "Release ${tagName}"

  # 推送到远程仓库
  git push
  git push --tags
`);
