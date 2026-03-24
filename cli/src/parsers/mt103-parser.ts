import * as fs from 'fs';
import { PaymentRecord, MT103Message } from '../types';

/**
 * SWIFT MT103 message parser for bank integration.
 * Parses MT103 single customer credit transfer messages and converts
 * them to PaymentRecord format for batch processing.
 *
 * MT103 field mappings:
 *   :20:  - Transaction Reference
 *   :23B: - Bank Operation Code
 *   :32A: - Value Date / Currency / Amount
 *   :50K: - Ordering Customer (sender)
 *   :52A: - Ordering Institution (sender BIC)
 *   :57A: - Account With Institution (receiver BIC)
 *   :59:  - Beneficiary Customer (receiver)
 *   :70:  - Remittance Information
 *   :71A: - Details of Charges
 */
export function parseMT103(filePath: string): PaymentRecord[] {
  const content = fs.readFileSync(filePath, 'utf-8');
  const messages = splitMT103Messages(content);
  return messages.map(convertMT103ToPayment);
}

function splitMT103Messages(content: string): MT103Message[] {
  const messageBlocks = content.split(/\{4:/);
  const messages: MT103Message[] = [];

  for (const block of messageBlocks) {
    if (!block.trim()) continue;

    const msg: MT103Message = {
      senderBIC: '',
      receiverBIC: '',
      transactionRef: '',
      valueDate: '',
      currency: '',
      amount: '',
      orderingCustomer: '',
      beneficiaryCustomer: '',
      remittanceInfo: '',
    };

    const refMatch = block.match(/:20:(.+?)(?:\r?\n|$)/);
    if (refMatch) msg.transactionRef = refMatch[1].trim();

    const valueMatch = block.match(/:32A:(\d{6})([A-Z]{3})([0-9,.]+)/);
    if (valueMatch) {
      msg.valueDate = valueMatch[1];
      msg.currency = valueMatch[2];
      msg.amount = valueMatch[3].replace(',', '.');
    }

    const senderBICMatch = block.match(/:52A:(.+?)(?:\r?\n|$)/);
    if (senderBICMatch) msg.senderBIC = senderBICMatch[1].trim();

    const receiverBICMatch = block.match(/:57A:(.+?)(?:\r?\n|$)/);
    if (receiverBICMatch) msg.receiverBIC = receiverBICMatch[1].trim();

    const orderingMatch = block.match(/:50K:[\s\S]*?\n(.+?)(?:\r?\n:|$)/);
    if (orderingMatch) msg.orderingCustomer = orderingMatch[1].trim();

    const beneficiaryMatch = block.match(/:59:.*?\n?([A-Z0-9]{56}|G[A-Z0-9]{55})/);
    if (beneficiaryMatch) {
      msg.beneficiaryCustomer = beneficiaryMatch[1].trim();
    } else {
      const beneficiaryFallback = block.match(/:59:(.+?)(?:\r?\n:|$)/s);
      if (beneficiaryFallback) msg.beneficiaryCustomer = beneficiaryFallback[1].trim().split('\n')[0];
    }

    const remittanceMatch = block.match(/:70:(.+?)(?:\r?\n:|$)/s);
    if (remittanceMatch) msg.remittanceInfo = remittanceMatch[1].trim();

    if (msg.beneficiaryCustomer && msg.amount) {
      messages.push(msg);
    }
  }

  return messages;
}

function convertMT103ToPayment(msg: MT103Message): PaymentRecord {
  const assetMap: Record<string, string> = {
    USD: 'USDC',
    EUR: 'EURC',
    GBP: 'GBP',
    MXN: 'MXN',
    BRL: 'BRL',
    NGN: 'NGN',
    KES: 'KES',
    PHP: 'PHP',
    INR: 'INR',
    JPY: 'JPY',
  };

  return {
    destination: msg.beneficiaryCustomer,
    amount: msg.amount,
    asset: assetMap[msg.currency] || msg.currency || 'USDC',
    asset_issuer: '',
    memo: msg.remittanceInfo || msg.transactionRef || '',
    escrow_duration: 86400, // Default 24h escrow for SWIFT transfers
  };
}
