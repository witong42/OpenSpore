const { exec } = require('child_process');

module.exports = async (directory) => {
  return new Promise((resolve, reject) => {
    exec(`bun run dev`, { cwd: directory }, (error, stdout, stderr) => {
      if (error) {
        console.error(`Error: ${error.message}`);
        return resolve(JSON.stringify({ success: false, error: error.message }));
      }
      if (stderr) {
        console.error(`Stderr: ${stderr}`);
      }

      const readyPattern = /Ready in/;
      if (readyPattern.test(stdout)) {
        console.log('Next.js development server started successfully.');
        resolve(JSON.stringify({ success: true, message: 'Next.js development server started successfully.' }));
      } else {
        console.log('Next.js development server may have started with errors.');
        resolve(JSON.stringify({ success: false, message: 'Next.js development server may have started with errors.' }));
      }
    });
  });
};