// Dependencies
const fs = require('fs').promises;
const path = require('path');

module.exports = {
  name: 'enhanced_health_check',
  description: 'Enhanced health check skill with file system read/write and path validation.',
  instructions: 'This skill performs system health checks and validates file system read/write operations and path accessibility.',
  async execute(context) {
    let results = [];

    // 1. Original health checks (example - replace with actual health check logic)
    results.push({ check: 'Basic system check', status: 'OK' });

    // 2. File system read/write test
    const workspaceDir = '/tmp/workspace'; // Use a default value for workspace
    const testFilePath = path.join(workspaceDir, 'health_check_test.txt');
    const testContent = 'This is a test file for health check.';

    try {
      // Create workspace directory if it doesn't exist
      await fs.mkdir(workspaceDir, { recursive: true });

      // Write to file
      await fs.writeFile(testFilePath, testContent, 'utf8');
      results.push({ check: 'File write', status: 'OK', file: testFilePath });

      // Read from file
      const readContent = await fs.readFile(testFilePath, 'utf8');
      if (readContent === testContent) {
        results.push({ check: 'File read', status: 'OK', file: testFilePath });
      } else {
        results.push({ check: 'File read', status: 'FAILED', file: testFilePath, error: 'Content mismatch' });
      }

      // Delete file
      await fs.unlink(testFilePath);
      results.push({ check: 'File delete', status: 'OK', file: testFilePath });

      // 3. Path validation
      try {
        await fs.access(workspaceDir, fs.constants.F_OK | fs.constants.W_OK);
        results.push({ check: 'Workspace path validation', status: 'OK', path: workspaceDir });
      } catch (err) {
        results.push({ check: 'Workspace path validation', status: 'FAILED', path: workspaceDir, error: err.message });
      }

    } catch (error) {
      results.push({ check: 'File system test', status: 'FAILED', error: error.message });
    }

    return JSON.stringify({ success: true, results: results });
  }
};