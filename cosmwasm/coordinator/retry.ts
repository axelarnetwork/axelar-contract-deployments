import { printInfo, printWarn } from '../../common';

export class RetryManager {
    public static async withRetry<T>(operation: () => Promise<T>): Promise<T> {
        const maxRetries = 3;
        const retryDelay = 1000;

        printInfo(`Retry configuration: max ${maxRetries} attempts, ${retryDelay}ms delay`);

        for (let attempt = 1; attempt <= maxRetries; attempt++) {
            try {
                return await operation();
            } catch (error) {
                if (attempt === maxRetries) {
                    throw error;
                }

                printWarn(
                    `Operation failed (attempt ${attempt}/${maxRetries}), retrying... Error: ${error.message}, next attempt in ${retryDelay}ms`,
                );

                await new Promise((resolve) => setTimeout(resolve, retryDelay));
            }
        }
    }
}
