'use strict';

const { outputJsonSync } = require('fs-extra');
const chalk = require('chalk');
const https = require('https');
const http = require('http');

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

const printInfo = (msg, info = '', colour = chalk.green) => {
    if (info) {
        console.log(`${msg}: ${colour(info)}\n`);
    } else {
        console.log(`${msg}\n`);
    }
};

const printWarn = (msg, info = '') => {
    if (info) {
        msg = `${msg}: ${info}`;
    }

    console.log(`${chalk.italic.yellow(msg)}\n`);
};

const printError = (msg, info = '') => {
    if (info) {
        msg = `${msg}: ${info}`;
    }

    console.log(`${chalk.bold.red(msg)}\n`);
};

function printLog(log) {
    console.log(JSON.stringify({ log }, null, 2));
}

const isNonEmptyString = (arg) => {
    return typeof arg === 'string' && arg !== '';
};

const isString = (arg) => {
    return typeof arg === 'string';
};

const isStringArray = (arr) => Array.isArray(arr) && arr.every(isString);

const isNumber = (arg) => {
    return Number.isInteger(arg);
};

const isValidNumber = (arg) => {
    return !isNaN(parseInt(arg)) && isFinite(arg);
};

const isValidDecimal = (arg) => {
    return !isNaN(parseFloat(arg)) && isFinite(arg);
};

const isNumberArray = (arr) => {
    if (!Array.isArray(arr)) {
        return false;
    }

    for (const item of arr) {
        if (!isNumber(item)) {
            return false;
        }
    }

    return true;
};

const isNonEmptyStringArray = (arr) => {
    if (!Array.isArray(arr)) {
        return false;
    }

    for (const item of arr) {
        if (typeof item !== 'string') {
            return false;
        }
    }

    return true;
};

function copyObject(obj) {
  return JSON.parse(JSON.stringify(obj));
}

const httpGet = (url) => {
  return new Promise((resolve, reject) => {
      (url.startsWith('https://') ? https : http).get(url, (res) => {
          const { statusCode } = res;
          const contentType = res.headers['content-type'];
          let error;

          if (statusCode !== 200 && statusCode !== 301) {
              error = new Error('Request Failed.\n' + `Request: ${url}\nStatus Code: ${statusCode}`);
          } else if (!/^application\/json/.test(contentType)) {
              error = new Error('Invalid content-type.\n' + `Expected application/json but received ${contentType}`);
          }

          if (error) {
              res.resume();
              reject(error);
              return;
          }

          res.setEncoding('utf8');
          let rawData = '';
          res.on('data', (chunk) => {
              rawData += chunk;
          });
          res.on('end', () => {
              try {
                  const parsedData = JSON.parse(rawData);
                  resolve(parsedData);
              } catch (e) {
                  reject(e);
              }
          });
      });
  });
};

const httpPost = async (url, data) => {
  const response = await fetch(url, {
      method: 'POST',
      headers: {
          'Content-Type': 'application/json',
      },
      body: JSON.stringify(data),
  });
  return response.json();
}

module.exports = {
    loadConfig,
    saveConfig,
    writeJSON,
    printInfo,
    printWarn,
    printError,
    printLog,
    isNonEmptyString,
    isString,
    isStringArray,
    isNumber,
    isValidNumber,
    isValidDecimal,
    isNumberArray,
    isNonEmptyStringArray,
    copyObject,
    httpGet,
    httpPost,
};
