import { useState, useEffect, useCallback } from 'react';
import { StellarCrossBorderSDK } from '@stellar-cross-border/sdk';
import { 
  PaymentStatus, 
  EscrowStatus, 
  PaymentRequest, 
  PaymentOptions,
  ExchangeRateRequest,
  ComplianceRequest,
  EscrowCreationResult,
  TransactionResult,
  ExchangeRateResult,
  ComplianceCheckResult
} from '@stellar-cross-border/sdk';

interface UseStellarPaymentOptions {
  autoRefresh?: boolean;
  refreshInterval?: number;
}

interface StellarPaymentState {
  loading: boolean;
  error: string | null;
  paymentStatus: PaymentStatus | null;
  exchangeRate: ExchangeRateResult | null;
  complianceCheck: ComplianceCheckResult | null;
}

interface StellarPaymentActions {
  createPayment: (request: PaymentRequest, options?: PaymentOptions) => Promise<EscrowCreationResult>;
  releaseEscrow: (escrowId: string, signer: any, options?: PaymentOptions) => Promise<TransactionResult>;
  refundEscrow: (escrowId: string, signer: any, options?: PaymentOptions) => Promise<TransactionResult>;
  disputeEscrow: (escrowId: string, challenger: string, reason: string, evidence: Uint8Array, signer: any, options?: PaymentOptions) => Promise<TransactionResult>;
  getExchangeRate: (request: ExchangeRateRequest) => Promise<ExchangeRateResult>;
  checkCompliance: (request: ComplianceRequest) => Promise<ComplianceCheckResult>;
  getPaymentStatus: (escrowId: string) => Promise<PaymentStatus>;
  refreshStatus: () => Promise<void>;
  clearError: () => void;
}

export const useStellarPayment = (
  sdk: StellarCrossBorderSDK,
  escrowId?: string,
  options: UseStellarPaymentOptions = {}
): StellarPaymentState & StellarPaymentActions => {
  const { autoRefresh = true, refreshInterval = 30000 } = options;

  const [state, setState] = useState<StellarPaymentState>({
    loading: false,
    error: null,
    paymentStatus: null,
    exchangeRate: null,
    complianceCheck: null,
  });

  const setLoading = (loading: boolean) => {
    setState(prev => ({ ...prev, loading }));
  };

  const setError = (error: string | null) => {
    setState(prev => ({ ...prev, error }));
  };

  const clearError = useCallback(() => {
    setError(null);
  }, []);

  const getPaymentStatus = useCallback(async (id: string): Promise<PaymentStatus> => {
    try {
      setLoading(true);
      const status = await sdk.paymentsInstance.getPaymentStatus(id);
      setState(prev => ({ ...prev, paymentStatus: status }));
      return status;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to get payment status';
      setError(errorMessage);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [sdk]);

  const createPayment = useCallback(async (
    request: PaymentRequest,
    options?: PaymentOptions
  ): Promise<EscrowCreationResult> => {
    try {
      setLoading(true);
      clearError();
      
      const result = await sdk.paymentsInstance.createPayment(request, options);
      
      if (result.success && result.escrowId) {
        await getPaymentStatus(result.escrowId);
      }
      
      return result;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to create payment';
      setError(errorMessage);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [sdk, getPaymentStatus, clearError]);

  const releaseEscrow = useCallback(async (
    id: string,
    signer: any,
    options?: PaymentOptions
  ): Promise<TransactionResult> => {
    try {
      setLoading(true);
      clearError();
      
      const result = await sdk.paymentsInstance.releaseEscrow(id, signer, options);
      
      if (result.success) {
        await getPaymentStatus(id);
      }
      
      return result;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to release escrow';
      setError(errorMessage);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [sdk, getPaymentStatus, clearError]);

  const refundEscrow = useCallback(async (
    id: string,
    signer: any,
    options?: PaymentOptions
  ): Promise<TransactionResult> => {
    try {
      setLoading(true);
      clearError();
      
      const result = await sdk.paymentsInstance.refundEscrow(id, signer, options);
      
      if (result.success) {
        await getPaymentStatus(id);
      }
      
      return result;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to refund escrow';
      setError(errorMessage);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [sdk, getPaymentStatus, clearError]);

  const disputeEscrow = useCallback(async (
    id: string,
    challenger: string,
    reason: string,
    evidence: Uint8Array,
    signer: any,
    options?: PaymentOptions
  ): Promise<TransactionResult> => {
    try {
      setLoading(true);
      clearError();
      
      const result = await sdk.paymentsInstance.disputeEscrow(
        id, 
        challenger, 
        reason, 
        evidence, 
        signer, 
        options
      );
      
      if (result.success) {
        await getPaymentStatus(id);
      }
      
      return result;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to dispute escrow';
      setError(errorMessage);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [sdk, getPaymentStatus, clearError]);

  const getExchangeRate = useCallback(async (
    request: ExchangeRateRequest
  ): Promise<ExchangeRateResult> => {
    try {
      setLoading(true);
      clearError();
      
      const result = await sdk.paymentsInstance.getExchangeRate(request);
      setState(prev => ({ ...prev, exchangeRate: result }));
      
      return result;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to get exchange rate';
      setError(errorMessage);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [sdk, clearError]);

  const checkCompliance = useCallback(async (
    request: ComplianceRequest
  ): Promise<ComplianceCheckResult> => {
    try {
      setLoading(true);
      clearError();
      
      const result = await sdk.paymentsInstance.checkCompliance(request);
      setState(prev => ({ ...prev, complianceCheck: result }));
      
      return result;
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to check compliance';
      setError(errorMessage);
      throw err;
    } finally {
      setLoading(false);
    }
  }, [sdk, clearError]);

  const refreshStatus = useCallback(async () => {
    if (escrowId) {
      await getPaymentStatus(escrowId);
    }
  }, [escrowId, getPaymentStatus]);

  // Auto-refresh payment status for pending escrows
  useEffect(() => {
    if (!autoRefresh || !escrowId || !state.paymentStatus) {
      return;
    }

    if (state.paymentStatus.status === EscrowStatus.Pending) {
      const interval = setInterval(() => {
        refreshStatus();
      }, refreshInterval);

      return () => clearInterval(interval);
    }
  }, [autoRefresh, escrowId, state.paymentStatus, refreshInterval, refreshStatus]);

  // Initial load of payment status if escrowId is provided
  useEffect(() => {
    if (escrowId) {
      getPaymentStatus(escrowId);
    }
  }, [escrowId, getPaymentStatus]);

  return {
    ...state,
    createPayment,
    releaseEscrow,
    refundEscrow,
    disputeEscrow,
    getExchangeRate,
    checkCompliance,
    getPaymentStatus,
    refreshStatus,
    clearError,
  };
};

export default useStellarPayment;
