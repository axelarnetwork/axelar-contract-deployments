import { printInfo, prompt } from '../../common';
import { encodeExecuteContractProposal, initContractConfig } from '../utils';
import { ConfigManager } from './config';
import type { RegisterDeploymentOptions, RegisterProtocolOptions } from './option-processor';
import { ProposalManager } from './proposal-manager';

export interface RegisterDeploymentMsg {
    register_deployment: {
        deployment_name: string;
    };
}

export class GovernanceManager {
    public configManager: ConfigManager;
    private proposalManager: ProposalManager;

    constructor(configManager: ConfigManager, proposalManager: ProposalManager) {
        this.configManager = configManager;
        this.proposalManager = proposalManager;
    }

    public async registerProtocol(processedOptions: RegisterProtocolOptions): Promise<void> {
        printInfo('Preparing register protocol proposal...');
        printInfo(`Environment: ${this.configManager.getEnvironment()}`);

        initContractConfig(this.configManager.getFullConfig(), { contractName: 'Coordinator', chainName: undefined });

        const serviceRegistryAddress = this.configManager.getContractAddressFromConfig('ServiceRegistry');
        const routerAddress = this.configManager.getContractAddressFromConfig('Router');
        const multisigAddress = this.configManager.getContractAddressFromConfig('Multisig');

        printInfo(`Service registry address: ${serviceRegistryAddress}`);
        printInfo(`Router address: ${routerAddress}`);
        printInfo(`Multisig address: ${multisigAddress}`);

        const message = {
            register_protocol: {
                service_registry_address: serviceRegistryAddress,
                router_address: routerAddress,
                multisig_address: multisigAddress,
            },
        };
        const messageJson = JSON.stringify(message, null, 2);

        printInfo('Generated register protocol message:', messageJson);

        const title = 'Register Protocol Contracts';
        const description = 'Register ServiceRegistry, Router, and Multisig contracts with Coordinator';

        printInfo('Creating governance proposal...');
        const proposal = encodeExecuteContractProposal(
            this.configManager.getFullConfig(),
            {
                ...processedOptions,
                contractName: 'Coordinator',
                msg: messageJson,
                title,
                description,
            },
            undefined,
        );

        if (prompt('Proceed with register protocol proposal submission?', processedOptions.yes)) {
            printInfo('Register protocol proposal submission cancelled');
            return;
        }

        printInfo('Submitting register protocol proposal...');
        const proposalId = await this.proposalManager.submitProposal(proposal, processedOptions.mnemonic, processedOptions.deposit);
        printInfo(`Register protocol proposal submitted successfully with ID: ${proposalId}`);

        this.configManager.saveConfig();
    }

    public async registerDeployment(processedOptions: RegisterDeploymentOptions): Promise<void> {
        printInfo('Preparing register deployment proposal...');
        printInfo(`Environment: ${this.configManager.getEnvironment()}`);
        printInfo(`Chain: ${processedOptions.chainName}`);

        const deploymentName = this.configManager.getDeploymentNameFromConfig(processedOptions.chainName);

        printInfo(`Using deployment name from config: ${deploymentName}`);

        if (prompt('Is the instantiation proposal executed?', processedOptions.yes)) {
            printInfo('Instantiation proposal extraction is not finished yet, please wait for it to be executed');
            return;
        }

        const instantiationProposalId = this.configManager.getInstantiationProposalIdFromConfig(processedOptions.chainName);
        if (instantiationProposalId) {
            const client = await this.proposalManager.getClient(processedOptions.mnemonic);
            await this.fetchAddressesFromCoordinator(client, deploymentName);
        } else {
            throw new Error('No instantiation proposal ID found in config, skipping event extraction');
        }

        const message: RegisterDeploymentMsg = {
            register_deployment: {
                deployment_name: deploymentName,
            },
        };
        const messageJson = JSON.stringify(message, null, 2);

        printInfo('Generated register deployment message:', messageJson);

        const title = `Register Deployment for ${deploymentName}`;
        const description = `Register deployment with name ${deploymentName} with Coordinator`;
        const proposal = encodeExecuteContractProposal(
            this.configManager.getFullConfig(),
            {
                ...processedOptions,
                contractName: 'Coordinator',
                msg: messageJson,
                title,
                description,
            },
            undefined,
        );

        if (prompt('Proceed with register deployment proposal submission?', processedOptions.yes)) {
            printInfo('Register deployment proposal submission cancelled');
            return;
        }

        const proposalId = await this.proposalManager.submitProposal(proposal, processedOptions.mnemonic, processedOptions.deposit);
        printInfo(`Register deployment proposal submitted successfully with ID: ${proposalId}`);

        this.configManager.saveConfig();
    }

    private async fetchAddressesFromCoordinator(client: unknown, deploymentName: string): Promise<void> {
        // TODO tkulik: Implement fetching Gateway, VotingVerifier, and MultisigProver addresses from Coordinator
        //              based on the deployment name.
        //              This point requires action from protocol team.
        //              The addresses should be saved to the config.
    }
}
