import React, { useState, useCallback } from 'react';
import { useForm } from 'react-hook-form';
import { toast } from 'react-hot-toast';
import { 
  ArrowRight, 
  Clock, 
  Shield, 
  DollarSign, 
  CheckCircle, 
  AlertCircle,
  Loader2 
} from 'lucide-react';
import { StellarCrossBorderSDK } from '@stellar-cross-border/sdk';
import { PaymentRequest, PaymentOptions } from '@stellar-cross-border/sdk';

interface PaymentFormData {
  from: string;
  to: string;
  amount: string;
  token: string;
  releaseTime: string;
  memo?: string;
  feeBump: boolean;
}

interface PaymentFormProps {
  sdk: StellarCrossBorderSDK;
  onSuccess?: (result: any) => void;
  onError?: (error: string) => void;
}

export const PaymentForm: React.FC<PaymentFormProps> = ({ 
  sdk, 
  onSuccess, 
  onError 
}) => {
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [currentStep, setCurrentStep] = useState(1);
  const [paymentResult, setPaymentResult] = useState<any>(null);

  const {
    register,
    handleSubmit,
    watch,
    formState: { errors, isValid },
    setValue,
    getValues,
  } = useForm<PaymentFormData>({
    mode: 'onChange',
    defaultValues: {
      feeBump: true,
      releaseTime: '24',
    },
  });

  const watchedValues = watch();

  const validateStellarAddress = (address: string) => {
    try {
      return sdk.clientInstance.validateAddress(address);
    } catch {
      return false;
    }
  };

  const onSubmit = useCallback(async (data: PaymentFormData) => {
    if (!validateStellarAddress(data.from)) {
      toast.error('Invalid sender address');
      return;
    }

    if (!validateStellarAddress(data.to)) {
      toast.error('Invalid receiver address');
      return;
    }

    setIsSubmitting(true);
    setCurrentStep(2);

    try {
      const releaseTime = Math.floor(Date.now() / 1000) + (parseInt(data.releaseTime) * 3600);
      
      const paymentRequest: PaymentRequest = {
        from: data.from,
        to: data.to,
        amount: data.amount,
        token: data.token,
        release_time: releaseTime,
        metadata: {},
      };

      const paymentOptions: PaymentOptions = {
        feeBump: data.feeBump,
        memo: data.memo,
        submit: true,
      };

      const result = await sdk.paymentsInstance.createPayment(
        paymentRequest,
        paymentOptions
      );

      if (result.success) {
        setPaymentResult(result);
        setCurrentStep(3);
        onSuccess?.(result);
        toast.success('Payment created successfully!');
      } else {
        throw new Error(result.error || 'Payment creation failed');
      }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      onError?.(errorMessage);
      toast.error(errorMessage);
      setCurrentStep(1);
    } finally {
      setIsSubmitting(false);
    }
  }, [sdk, onSuccess, onError]);

  const resetForm = () => {
    setCurrentStep(1);
    setPaymentResult(null);
    setIsSubmitting(false);
  };

  const commonTokens = [
    { symbol: 'XLM', name: 'Stellar Lumens', contract: 'native' },
    { symbol: 'USDC', name: 'USD Coin', contract: 'GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFLBKYXRH7CL5BJM4A3' },
    { symbol: 'EURC', name: 'Euro Coin', contract: 'GDZQJFSYNKSWDYM7KKEGZUPXNBNLAO5FQMJGFJFD7ZMPGA2U6WMR5VY' },
  ];

  return (
    <div className="max-w-2xl mx-auto bg-white rounded-lg shadow-lg p-6">
      <div className="mb-6">
        <h2 className="text-2xl font-bold text-gray-900 mb-2">
          Cross-Border Payment
        </h2>
        <p className="text-gray-600">
          Create a secure escrow payment with built-in compliance checks
        </p>
      </div>

      <div className="mb-6">
        <div className="flex items-center justify-between">
          {[1, 2, 3].map((step) => (
            <div key={step} className="flex items-center">
              <div
                className={`w-8 h-8 rounded-full flex items-center justify-center text-sm font-medium ${
                  currentStep >= step
                    ? 'bg-blue-600 text-white'
                    : 'bg-gray-200 text-gray-600'
                }`}
              >
                {step}
              </div>
              {step < 3 && (
                <div
                  className={`w-16 h-1 mx-2 ${
                    currentStep > step ? 'bg-blue-600' : 'bg-gray-200'
                  }`}
                />
              )}
            </div>
          ))}
        </div>
        <div className="flex justify-between mt-2 text-xs text-gray-600">
          <span>Details</span>
          <span>Processing</span>
          <span>Complete</span>
        </div>
      </div>

      {currentStep === 1 && (
        <form onSubmit={handleSubmit(onSubmit)} className="space-y-6">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                From Address
              </label>
              <input
                {...register('from', {
                  required: 'Sender address is required',
                  validate: validateStellarAddress,
                })}
                type="text"
                placeholder="G..."
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                disabled={isSubmitting}
              />
              {errors.from && (
                <p className="mt-1 text-sm text-red-600">{errors.from.message}</p>
              )}
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                To Address
              </label>
              <input
                {...register('to', {
                  required: 'Receiver address is required',
                  validate: validateStellarAddress,
                })}
                type="text"
                placeholder="G..."
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                disabled={isSubmitting}
              />
              {errors.to && (
                <p className="mt-1 text-sm text-red-600">{errors.to.message}</p>
              )}
            </div>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                Amount
              </label>
              <div className="relative">
                <DollarSign className="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400 w-4 h-4" />
                <input
                  {...register('amount', {
                    required: 'Amount is required',
                    pattern: {
                      value: /^\d+(\.\d{1,7})?$/,
                      message: 'Invalid amount format',
                    },
                  })}
                  type="text"
                  placeholder="0.00"
                  className="w-full pl-10 pr-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                  disabled={isSubmitting}
                />
              </div>
              {errors.amount && (
                <p className="mt-1 text-sm text-red-600">{errors.amount.message}</p>
              )}
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                Token
              </label>
              <select
                {...register('token', { required: 'Token is required' })}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                disabled={isSubmitting}
              >
                <option value="">Select token</option>
                {commonTokens.map((token) => (
                  <option key={token.symbol} value={token.contract}>
                    {token.symbol} - {token.name}
                  </option>
                ))}
              </select>
              {errors.token && (
                <p className="mt-1 text-sm text-red-600">{errors.token.message}</p>
              )}
            </div>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                Release Time (hours)
              </label>
              <div className="relative">
                <Clock className="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400 w-4 h-4" />
                <input
                  {...register('releaseTime', {
                    required: 'Release time is required',
                    min: { value: 1, message: 'Minimum 1 hour' },
                    max: { value: 168, message: 'Maximum 168 hours (7 days)' },
                  })}
                  type="number"
                  placeholder="24"
                  className="w-full pl-10 pr-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                  disabled={isSubmitting}
                />
              </div>
              {errors.releaseTime && (
                <p className="mt-1 text-sm text-red-600">{errors.releaseTime.message}</p>
              )}
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                Memo (optional)
              </label>
              <input
                {...register('memo')}
                type="text"
                placeholder="Payment description"
                maxLength={28}
                className="w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-blue-500"
                disabled={isSubmitting}
              />
            </div>
          </div>

          <div className="flex items-center space-x-2">
            <input
              {...register('feeBump')}
              type="checkbox"
              id="feeBump"
              className="w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500"
              disabled={isSubmitting}
            />
            <label htmlFor="feeBump" className="text-sm text-gray-700">
              Use fee bump transaction (recommended for cross-border)
            </label>
          </div>

          <div className="bg-blue-50 border border-blue-200 rounded-md p-4">
            <div className="flex items-start space-x-2">
              <Shield className="w-5 h-5 text-blue-600 mt-0.5" />
              <div className="text-sm text-blue-800">
                <p className="font-medium mb-1">Security Features</p>
                <ul className="list-disc list-inside space-y-1 text-blue-700">
                  <li>Time-locked escrow protects both parties</li>
                  <li>Automatic compliance checks</li>
                  <li>Dispute resolution mechanism</li>
                  <li>Fee bump ensures transaction completion</li>
                </ul>
              </div>
            </div>
          </div>

          <button
            type="submit"
            disabled={!isValid || isSubmitting}
            className="w-full bg-blue-600 text-white py-2 px-4 rounded-md hover:bg-blue-700 disabled:bg-gray-400 disabled:cursor-not-allowed transition-colors duration-200 flex items-center justify-center space-x-2"
          >
            {isSubmitting ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                <span>Processing...</span>
              </>
            ) : (
              <>
                <ArrowRight className="w-4 h-4" />
                <span>Create Payment</span>
              </>
            )}
          </button>
        </form>
      )}

      {currentStep === 2 && (
        <div className="text-center py-8">
          <Loader2 className="w-12 h-12 animate-spin text-blue-600 mx-auto mb-4" />
          <h3 className="text-lg font-medium text-gray-900 mb-2">
            Processing Payment
          </h3>
          <p className="text-gray-600">
            Creating escrow and running compliance checks...
          </p>
        </div>
      )}

      {currentStep === 3 && paymentResult && (
        <div className="text-center py-8">
          <CheckCircle className="w-12 h-12 text-green-600 mx-auto mb-4" />
          <h3 className="text-lg font-medium text-gray-900 mb-2">
            Payment Created Successfully!
          </h3>
          <div className="bg-gray-50 rounded-md p-4 mb-6 text-left">
            <div className="space-y-2">
              <div>
                <span className="text-sm font-medium text-gray-700">Transaction Hash:</span>
                <p className="text-sm text-gray-600 font-mono break-all">
                  {paymentResult.hash}
                </p>
              </div>
              <div>
                <span className="text-sm font-medium text-gray-700">Escrow ID:</span>
                <p className="text-sm text-gray-600 font-mono break-all">
                  {paymentResult.escrowId}
                </p>
              </div>
            </div>
          </div>
          <button
            onClick={resetForm}
            className="bg-blue-600 text-white py-2 px-6 rounded-md hover:bg-blue-700 transition-colors duration-200"
          >
            Create Another Payment
          </button>
        </div>
      )}
    </div>
  );
};
