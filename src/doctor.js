global.EXEC = async (command) => { console.log('EXEC called with: ' + command); return await new Promise(resolve => { setTimeout(() => { resolve(''); }, 100); }); };
global.SUBMIT_SKILL = async (skill) => { console.log('SUBMIT_SKILL called with: ' + skill); return await new Promise(resolve => { setTimeout(() => { resolve(''); }, 100); }); };
console.log('Running doctor.js...');

async function main() {
  let execStatus = false;
  try {
    const command = 'echo \\\"exec test\\\"';
    const result = await EXEC(command);
    if (result && result.includes('exec test')) {
      console.log('EXEC tool: OK');
      execStatus = true;
    } else {
      console.error('EXEC tool: FAIL - unexpected output:', result);
    }
  } catch (error) {
    console.error('EXEC tool: FAIL -', error);
  }

  let submitSkillStatus = false;
  try {
    const filename = 'test_skill.js';
    const code = 'console.log(\\\"submit_skill test\\\");';
    const submitSkillResult = await SUBMIT_SKILL(filename + '|||' + code);
    if (submitSkillResult) {
      console.log('SUBMIT_SKILL tool: OK');
      submitSkillStatus = true;
    } else {
      console.error('SUBMIT_SKILL tool: FAIL - no result');
    }
  } catch (error) {
    console.error('SUBMIT_SKILL tool: FAIL -', error);
  }

  if (execStatus && submitSkillStatus) {
    console.log('All checks passed!');
  } else {
    console.error('Some checks failed.');
  }
}

main();

async function pruneContext() {
  try {
    const command = 'cargo run --release -- prune-context';
    const result = await EXEC(command);
    console.log(\`Context pruning result: ${result}\`);
  } catch (error) {
    console.error(\`Error pruning context: ${error}\`);
  }
}

await pruneContext();