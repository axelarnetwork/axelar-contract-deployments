const { exec } = require('child_process');

const runCliCommand = (command) => {
    return new Promise((resolve, reject) => {
        exec(command, (error, stdout, stderr) => {
            if (error) {
                reject(new Error(`Command failed: ${stderr || error.message}`));
            } else {
                resolve(stdout);
            }
        });
    });
};

module.exports = {
    runCliCommand,
};
