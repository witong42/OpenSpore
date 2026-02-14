const { exec } = require('child_process');
const { promisify } = require('util');
const path = require('path');
const execAsync = promisify(exec);

/**
 * Modernized Clone Repo Skill
 * Handles both string and object arguments.
 */
module.exports = async (args) => {
  let repo_url = "";
  let clone_path = "workspace/repo-clone";

  // 1. Parse Arguments
  if (typeof args === 'string') {
    const parts = args.split(/\s+/);
    repo_url = parts[0].replace(/['"]/g, '');

    // Extract --path flag
    const pathMatch = args.match(/--path=["']?([^"'\s]+)["']?/);
    if (pathMatch) {
      clone_path = pathMatch[1];
    }
  } else if (args && typeof args === 'object') {
    repo_url = args.repo_url || args.url || "";
    clone_path = args.path || args.clone_path || clone_path;
  }

  if (!repo_url) {
    return JSON.stringify({ success: false, error: "No repository URL provided." });
  }

  // 2. Resolve Path (Relative to .openspore root)
  const appRoot = process.env.OPENSPORE_ROOT || path.join(process.env.HOME, '.openspore');
  const absolutePath = path.isAbsolute(clone_path) ? clone_path : path.join(appRoot, clone_path);

  try {
    console.log(`🚀 Modern Cloner: Preparing to clone ${repo_url}...`);

    // Ensure parent directory exists
    const parentDir = path.dirname(absolutePath);
    const { withRetry } = require('../utils');
    //Validate and Normalize path
    const normalizedPath = path.normalize(absolutePath);
    const isValidPath = normalizedPath.startsWith(appRoot);
    if (!isValidPath) {
      return JSON.stringify({ success: false, error: `Invalid clone path: ${normalizedPath}. Must be within ${appRoot}` });
    }

    await withRetry(() => execAsync(`mkdir -p "${parentDir}"`));

    // Execute Clone
    const { stdout, stderr } = await execAsync(`git clone "${repo_url}" "${absolutePath}"`);

    return JSON.stringify({
      success: true,
      message: "Repository cloned successfully.",
      path: absolutePath,
      stdout,
      stderr
    });
  } catch (error) {
    return JSON.stringify({
      success: false,
      error: error.message,
      stderr: error.stderr,
      stdout: error.stdout
    });
  }
};
