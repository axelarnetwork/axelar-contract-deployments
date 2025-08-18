import { ConfigManager, CoordinatorOptions, DEFAULTS } from '.';
import { calculateDomainSeparator, printInfo } from '../../common';

export class ChainConfigManager {
    private configManager: ConfigManager;

    constructor(configManager: ConfigManager) {
        this.configManager = configManager;
    }

    public updateChainConfig(chainName: string, options: CoordinatorOptions): void {
        printInfo(`Storing chain-specific parameters for ${chainName}...`);

        const chainConfig = this.configManager.getChainConfig(chainName);
        const governanceAddress = options.governanceAddress || this.configManager.getDefaultGovernanceAddress();
        const serviceName = options.serviceName || DEFAULTS.serviceName;
        const rewardsAddress = options.rewardsAddress || this.configManager.getContractAddressFromConfig('Rewards');
        const sourceGatewayAddress =
            options.sourceGatewayAddress || this.configManager.getContractAddressFromChainConfig(chainName, 'AxelarGateway');
        const domainSeparator =
            options.domainSeparator ||
            calculateDomainSeparator(chainName, this.configManager.getContractAddressFromConfig('Router'), chainConfig.axelarId);
        const votingVerifierParams = {
            governanceAddress,
            serviceName,
            sourceGatewayAddress,
            votingThreshold: [
                options.votingThreshold?.[0] || DEFAULTS.votingThreshold[0],
                options.votingThreshold?.[1] || DEFAULTS.votingThreshold[1],
            ],
            blockExpiry: options.blockExpiry || DEFAULTS.blockExpiry,
            confirmationHeight:
                typeof options.confirmationHeight === 'number'
                    ? options.confirmationHeight
                    : options.confirmationHeight
                      ? parseInt(options.confirmationHeight.toString())
                      : DEFAULTS.confirmationHeight,
            sourceChain: chainConfig.axelarId,
            rewardsAddress,
            msgIdFormat: options.msgIdFormat || DEFAULTS.msgIdFormat,
            addressFormat: options.addressFormat || DEFAULTS.addressFormat,
            contractAdmin: options.contractAdmin,
        };

        const multisigProverParams = {
            governanceAddress,
            multisigAddress: this.configManager.getContractAddressFromConfig('Multisig'),
            signingThreshold: [
                options.signingThreshold?.[0] || DEFAULTS.signingThreshold[0],
                options.signingThreshold?.[1] || DEFAULTS.signingThreshold[1],
            ],
            serviceName,
            chainName: chainConfig.axelarId,
            verifierSetDiffThreshold:
                typeof options.verifierSetDiffThreshold === 'number'
                    ? options.verifierSetDiffThreshold
                    : options.verifierSetDiffThreshold
                      ? parseInt(options.verifierSetDiffThreshold.toString())
                      : DEFAULTS.verifierSetDiffThreshold,
            encoder: options.encoder || DEFAULTS.encoder,
            keyType: options.keyType || DEFAULTS.keyType,
            domainSeparator: domainSeparator.replace('0x', ''),
            contractAdmin: options.contractAdmin,
            adminAddress: options.multisigAdmin,
        };

        const gatewayParams = {
            salt: options.salt,
            contractAdmin: options.contractAdmin,
        };

        const axelarContracts = this.configManager.getFullConfig().axelar?.contracts;
        if (!axelarContracts) {
            throw new Error('Axelar contracts section not found in config');
        }

        if (!axelarContracts.VotingVerifier) {
            axelarContracts.VotingVerifier = {};
        }
        if (!axelarContracts.MultisigProver) {
            axelarContracts.MultisigProver = {};
        }
        if (!axelarContracts.Gateway) {
            axelarContracts.Gateway = {};
        }

        (axelarContracts.VotingVerifier as Record<string, unknown>)[chainName] = votingVerifierParams;
        (axelarContracts.MultisigProver as Record<string, unknown>)[chainName] = multisigProverParams;
        (axelarContracts.Gateway as Record<string, unknown>)[chainName] = gatewayParams;

        this.configManager.saveConfig();

        printInfo('Chain-specific parameters stored successfully');
    }
}
