'use strict';

require('../common/cli-utils');

const { Option } = require('commander');

const addStoreOptions = (program) => {
    program.addOption(
        new Option(
            '-a, --artifact-dir <artifactDir>',
            'Path to the contract artifact directory to upload (required if --version is not used)',
        ).env('ARTIFACT_DIR'),
    );

    program.addOption(
        new Option(
            '-v, --version <contractVersion>',
            'Specify a released version (X.Y.Z) or a commit hash to upload (required if --artifact-dir (Stellar) is not used)',
        ).env('CONTRACT_VERSION'),
    );

    program.hook('preAction', async (thisCommand) => {
        const opts = thisCommand.opts();

        if (!opts.artifactDir && !opts.version) {
            throw new Error('Either --artifact-dir (Stellar) or --version is required');
        }
    });
};

module.exports = {
    addStoreOptions,
};
