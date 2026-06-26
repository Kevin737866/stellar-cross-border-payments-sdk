export const SUPPORTED_TOKENS = ['XLM', 'USDC', 'EURC', 'yXLM'] as const;
export type SupportedToken = typeof SUPPORTED_TOKENS[number];

export interface PaymentFormValues {
  sender:      string;
  receiver:    string;
  amount:      string;
  token:       string;
  releaseTime: number | string; // unix seconds or empty
}

export interface FormErrors {
  sender?:      string;
  receiver?:    string;
  amount?:      string;
  token?:       string;
  releaseTime?: string;
}

const MAX_RELEASE_TIME_DAYS = 365 * 5; // 5 years

export function validatePaymentForm(values: PaymentFormValues): FormErrors {
  const errors: FormErrors = {};

  // Stellar address validation (G... or C... for contracts)
  const addressRe = /^[GC][A-Z2-7]{55}$/;
  if (!values.sender)                      errors.sender   = 'Sender address is required.';
  else if (!addressRe.test(values.sender)) errors.sender   = 'Enter a valid Stellar address.';

  if (!values.receiver)                       errors.receiver = 'Receiver address is required.';
  else if (!addressRe.test(values.receiver))  errors.receiver = 'Enter a valid Stellar address.';
  else if (values.receiver === values.sender) errors.receiver = 'Receiver must differ from sender.';

  const amt = parseFloat(values.amount);
  if (!values.amount)      errors.amount = 'Amount is required.';
  else if (isNaN(amt))     errors.amount = 'Amount must be a number.';
  else if (amt <= 0)       errors.amount = 'Amount must be greater than zero.';
  else if (!/^\d+(\.\d{1,7})?$/.test(values.amount))
    errors.amount = 'Maximum 7 decimal places (stroop precision).';

  if (!values.token) {
    errors.token = 'Please select a token.';
  } else if (!(SUPPORTED_TOKENS as readonly string[])) {
    errors.token = `Unsupported token. Choose one of: ${SUPPORTED_TOKENS.join(', ')}.`;
  }

  if (values.releaseTime !== '' && values.releaseTime !== undefined) {
    const rt  = Number(values.releaseTime);
    const now = Math.floor(Date.now() / 1000);
    const max = now + MAX_RELEASE_TIME_DAYS * 24 * 60 * 60;

    if (isNaN(rt) || rt <= 0) {
      errors.releaseTime = 'Release time must be a positive number.';
    } else if (rt <= now) {
      errors.releaseTime = 'Release time must be in the future.';
    } else if (rt > max) {
      errors.releaseTime = `Release time cannot exceed ${MAX_RELEASE_TIME_DAYS / 365} years from now.`;
    }
  }

  return errors;
}

export function hasErrors(errors: FormErrors): boolean {
  return Object.keys(errors).length > 0;
}