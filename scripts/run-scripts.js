const { execSync } = require("child_process");

function runScripts(scriptsStr) {
  const scripts = scriptsStr
    .split("\n")
    .map((script) => script.trim())
    .filter((script) => script);
  for (const script of scripts) {
    try {
      // 如果以#开头，则console.log
      if (script.startsWith("#")) {
        console.log(`\n${script}`);
      } else {
        console.log(`$ ${script}`);
        execSync(script, { stdio: "inherit" });
      }
    } catch (err) {
      console.error(err);
      process.exit(1);
    }
  }
}

module.exports = {
  runScripts,
};
