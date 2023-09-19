const { readJsonSync } = require('fs-extra');

const readJSON = (filePath, require = false) => {
    let data;

    try {
        data = readJsonSync(filePath, 'utf8');
    } catch (err) {
        if (err.code === 'ENOENT' && !require) {
            return undefined;
        }

        throw err;
    }

    return data;
};

module.exports = {
    readJSON,
};
