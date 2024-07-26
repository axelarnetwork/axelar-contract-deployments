'use strict'

const { outputJsonSync } = require('fs-extra');

function loadConfig(env) {
  return require(`${__dirname}/../axelar-chains-config/info/${env}.json`);
}

function saveConfig(config, env) {
  writeJSON(config, `${__dirname}/../axelar-chains-config/info/${env}.json`);
}

const writeJSON = (data, name) => {
  outputJsonSync(name, data, {
      spaces: 2,
      EOL: '\n',
  });
};

module.exports = {
  loadConfig,
  saveConfig,
  writeJSON,
}
