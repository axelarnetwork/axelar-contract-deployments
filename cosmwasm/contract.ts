import { printInfo, prompt } from '../common';
import { mainProcessor } from './processor';
// import { execute } from './submit-proposal'; // causing pre-commit hook to fail. Add later
import { executeTransaction } from './utils';

const confirmDirectExecution = (options, messages, contractAddress) => {
    printInfo('Contract address', contractAddress);

    const msgs = Array.isArray(messages) ? messages : [messages];
    msgs.forEach((msg, index) => {
        const message = typeof msg === 'string' ? JSON.parse(msg) : msg;
        printInfo(`Message ${index + 1}/${msgs.length}`, JSON.stringify(message, null, 2));
    });

    if (prompt('Proceed with direct execution?', options.yes)) {
        return false;
    }
    return true;
};

const executeDirectly = async (client, config, options, contractAddress, msg, fee) => {
    const msgs = Array.isArray(msg) ? msg : [msg];

    for (let i = 0; i < msgs.length; i++) {
        const msgJson = msgs[i];
        const message = typeof msgJson === 'string' ? JSON.parse(msgJson) : msgJson;

        const { transactionHash } = await executeTransaction(client, contractAddress, message, fee);
        printInfo(`Transaction ${i + 1}/${msgs.length} executed`, transactionHash);
    }
};
