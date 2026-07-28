import { StellarClient } from './client';
import { Networks, Account, BASE_FEE, Keypair } from 'stellar-sdk';

const TEST_CONFIG = {
  horizonUrl: 'https://horizon-testnet.stellar.org',
  sorobanRpcUrl: 'https://rpc-futurenet.stellar.org',
  networkPassphrase: Networks.TESTNET,
};

const TEST_CONTRACTS = {
  escrow: 'CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA',
  rateOracle: 'CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA',
  compliance: 'CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA',
};

function createClient(): StellarClient {
  return new StellarClient(TEST_CONFIG, TEST_CONTRACTS);
}

describe('StellarClient buildTransaction', () => {
  it('uses the fee bump fee when requested', async () => {
    const client = createClient();
    const sourceAccount = new Account(Keypair.random().publicKey(), '1');
    const builder = await client.buildTransaction(sourceAccount, [], {
      feeBump: true,
    });

    expect(builder.baseFee).toBe('2000');
  });

  it('keeps the standard base fee when fee bump is not requested', async () => {
    const client = createClient();
    const sourceAccount = new Account(Keypair.random().publicKey(), '1');
    const builder = await client.buildTransaction(sourceAccount, [], {});

    expect(builder.baseFee).toBe(BASE_FEE);
  });
});

describe('StellarClient invokeContractMethod', () => {
  it('throws when source account has insufficient sequence', async () => {
    const client = createClient();
    const keypair = Keypair.random();
    const sourceAccount = new Account(keypair.publicKey(), '1');

    await expect(
      client.invokeContractMethod(
        TEST_CONTRACTS.escrow,
        'hello',
        [],
        sourceAccount,
        keypair
      )
    ).rejects.toThrow();
  });
});
