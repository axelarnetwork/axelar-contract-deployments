'use strict';

const fs = require('fs');
const path = require('path');
const { outputJsonSync } = require('fs-extra');
const chalk = require('chalk');
const https = require('https');
const http = require('http');
const readlineSync = require('readline-sync');

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
};

/**
 * Parses the input string into an array of arguments, recognizing and converting
 * to the following types: boolean, number, array, and string.
 *
 * @param {string} args - The string of arguments to parse.
 *
 * @returns {Array} - An array containing parsed arguments.
 *
 * @example
 * const input = "hello true 123 [1,2,3]";
 * const output = parseArgs(input);
 * console.log(output); // Outputs: [ 'hello', true, 123, [ 1, 2, 3] ]
 */
const parseArgs = (args) => {
    return args
        .split(/\s+/)
        .filter((item) => item !== '')
        .map((arg) => {
            if (arg.startsWith('[') && arg.endsWith(']')) {
                return JSON.parse(arg);
            } else if (arg === 'true') {
                return true;
            } else if (arg === 'false') {
                return false;
            } else if (!isNaN(arg) && !arg.startsWith('0x')) {
                return Number(arg);
            }

            return arg;
        });
};

function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}

function timeout(prom, time, exception) {
    let timer;

    // Racing the promise with a timer
    // If the timer resolves first, the promise is rejected with the exception
    return Promise.race([prom, new Promise((resolve, reject) => (timer = setTimeout(reject, time, exception)))]).finally(() =>
        clearTimeout(timer),
    );
}

/**
 * Validate if the input string matches the time format YYYY-MM-DDTHH:mm:ss
 *
 * @param {string} timeString - The input time string.
 * @return {boolean} - Returns true if the format matches, false otherwise.
 */
function isValidTimeFormat(timeString) {
    const regex = /^\d{4}-(?:0[1-9]|1[0-2])-(?:0[1-9]|1\d|2\d|3[01])T(?:[01]\d|2[0-3]):[0-5]\d:[0-5]\d$/;

    if (timeString === '0') {
        return true;
    }

    return regex.test(timeString);
}

const dateToEta = (utcTimeString) => {
    if (utcTimeString === '0') {
        return 0;
    }

    const date = new Date(utcTimeString + 'Z');

    if (isNaN(date.getTime())) {
        throw new Error(`Invalid date format provided: ${utcTimeString}`);
    }

    return Math.floor(date.getTime() / 1000);
};

const etaToDate = (timestamp) => {
    const date = new Date(timestamp * 1000);

    if (isNaN(date.getTime())) {
        throw new Error(`Invalid timestamp provided: ${timestamp}`);
    }

    return date.toISOString().slice(0, 19);
};

const getCurrentTimeInSeconds = () => {
    const now = new Date();
    const currentTimeInSecs = Math.floor(now.getTime() / 1000);
    return currentTimeInSecs;
};

/**
 * Prompt the user for confirmation
 * @param {string} question Prompt question
 * @param {boolean} yes If true, skip the prompt
 * @returns {boolean} Returns true if the prompt was skipped, false otherwise
 */
const prompt = (question, yes = false) => {
    // skip the prompt if yes was passed
    if (yes) {
        return false;
    }

    const answer = readlineSync.question(`${question} ${chalk.green('(y/n)')} `);
    console.log();

    return answer !== 'y';
};

function findProjectRoot(startDir) {
    let currentDir = startDir;

    while (currentDir !== path.parse(currentDir).root) {
        const potentialPackageJson = path.join(currentDir, 'package.json');

        if (fs.existsSync(potentialPackageJson)) {
            return currentDir;
        }

        currentDir = path.resolve(currentDir, '..');
    }

    throw new Error('Unable to find project root');
}

function toBigNumberString(number) {
    return Math.ceil(number).toLocaleString('en', { useGrouping: false });
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
    isValidTimeFormat,
    copyObject,
    httpGet,
    httpPost,
    parseArgs,
    sleep,
    dateToEta,
    etaToDate,
    getCurrentTimeInSeconds,
    prompt,
    findProjectRoot,
    toBigNumberString,
    timeout,
};
