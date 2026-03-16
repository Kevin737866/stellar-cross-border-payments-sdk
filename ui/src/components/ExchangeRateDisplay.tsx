import React, { useState, useEffect } from 'react';
import { toast } from 'react-hot-toast';
import {
  TrendingUp,
  TrendingDown,
  RefreshCw,
  DollarSign,
  Activity,
  Clock,
  Info,
  Loader2,
} from 'lucide-react';
import { StellarCrossBorderSDK } from '@stellar-cross-border/sdk';
import { ExchangeRateResult, ExchangeRateRequest } from '@stellar-cross-border/sdk';
import { format } from 'date-fns';

interface ExchangeRateDisplayProps {
  sdk: StellarCrossBorderSDK;
  fromCurrency: string;
  toCurrency: string;
  amount?: string;
  showHistorical?: boolean;
  autoRefresh?: boolean;
}

export const ExchangeRateDisplay: React.FC<ExchangeRateDisplayProps> = ({
  sdk,
  fromCurrency,
  toCurrency,
  amount = '1',
  showHistorical = false,
  autoRefresh = true,
}) => {
  const [rateData, setRateData] = useState<ExchangeRateResult | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [lastUpdated, setLastUpdated] = useState<Date | null>(null);

  const fetchExchangeRate = async () => {
    try {
      const request: ExchangeRateRequest = {
        from_currency: fromCurrency,
        to_currency: toCurrency,
      };

      const result = await sdk.paymentsInstance.getExchangeRate(request);
      setRateData(result);
      setLastUpdated(new Date());
      setError(null);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to fetch exchange rate';
      setError(errorMessage);
      toast.error(errorMessage);
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  };

  useEffect(() => {
    fetchExchangeRate();
    
    if (autoRefresh) {
      const interval = setInterval(() => {
        fetchExchangeRate();
      }, 60000); // Refresh every minute

      return () => clearInterval(interval);
    }
  }, [fromCurrency, toCurrency, autoRefresh]);

  const handleRefresh = async () => {
    setRefreshing(true);
    await fetchExchangeRate();
  };

  const calculateConvertedAmount = () => {
    if (!rateData || !amount) return '0';
    const rate = parseFloat(rateData.rate);
    const amountNum = parseFloat(amount);
    return (amountNum * rate).toFixed(7);
  };

  const formatRate = (rate: string) => {
    const num = parseFloat(rate);
    return num.toLocaleString('en-US', {
      minimumFractionDigits: 2,
      maximumFractionDigits: 7,
    });
  };

  const getTrendIcon = () => {
    if (!rateData || !showHistorical) return null;
    
    // This would typically compare with previous rate
    // For demo purposes, we'll show a random trend
    const isUp = Math.random() > 0.5;
    return isUp ? (
      <TrendingUp className="w-4 h-4 text-green-500" />
    ) : (
      <TrendingDown className="w-4 h-4 text-red-500" />
    );
  };

  const getConfidenceColor = (confidence: number) => {
    if (confidence >= 80) return 'text-green-600';
    if (confidence >= 60) return 'text-yellow-600';
    return 'text-red-600';
  };

  const getConfidenceLabel = (confidence: number) => {
    if (confidence >= 80) return 'High';
    if (confidence >= 60) return 'Medium';
    return 'Low';
  };

  if (loading) {
    return (
      <div className="max-w-2xl mx-auto bg-white rounded-lg shadow-lg p-6">
        <div className="flex items-center justify-center py-8">
          <Loader2 className="w-8 h-8 animate-spin text-blue-600" />
          <span className="ml-2 text-gray-600">Loading exchange rates...</span>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="max-w-2xl mx-auto bg-white rounded-lg shadow-lg p-6">
        <div className="flex items-center space-x-2 text-red-600">
          <Info className="w-5 h-5" />
          <span>{error}</span>
        </div>
        <button
          onClick={handleRefresh}
          className="mt-4 text-blue-600 hover:text-blue-700 flex items-center space-x-1"
        >
          <RefreshCw className="w-4 h-4" />
          <span>Retry</span>
        </button>
      </div>
    );
  }

  if (!rateData) {
    return null;
  }

  return (
    <div className="max-w-2xl mx-auto bg-white rounded-lg shadow-lg p-6">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-2xl font-bold text-gray-900">Exchange Rate</h2>
        <button
          onClick={handleRefresh}
          disabled={refreshing}
          className="flex items-center space-x-1 text-gray-600 hover:text-gray-800 disabled:opacity-50"
        >
          <RefreshCw className={`w-4 h-4 ${refreshing ? 'animate-spin' : ''}`} />
          <span>Refresh</span>
        </button>
      </div>

      <div className="bg-gradient-to-r from-blue-50 to-indigo-50 rounded-lg p-6 mb-6">
        <div className="text-center">
          <div className="flex items-center justify-center space-x-4 mb-4">
            <div className="text-3xl font-bold text-gray-900">{fromCurrency}</div>
            <div className="text-gray-400">→</div>
            <div className="text-3xl font-bold text-gray-900">{toCurrency}</div>
            {showHistorical && getTrendIcon()}
          </div>
          
          <div className="text-4xl font-bold text-blue-600 mb-2">
            {formatRate(rateData.rate)}
          </div>
          
          <div className="text-sm text-gray-600">
            1 {fromCurrency} = {formatRate(rateData.rate)} {toCurrency}
          </div>
        </div>
      </div>

      {amount !== '1' && (
        <div className="bg-gray-50 rounded-lg p-4 mb-6">
          <div className="flex items-center justify-between">
            <div>
              <div className="text-sm text-gray-600">Amount to Convert</div>
              <div className="text-xl font-semibold text-gray-900">
                {amount} {fromCurrency}
              </div>
            </div>
            <div className="text-right">
              <div className="text-sm text-gray-600">Converted Amount</div>
              <div className="text-xl font-semibold text-green-600">
                {calculateConvertedAmount()} {toCurrency}
              </div>
            </div>
          </div>
        </div>
      )}

      <div className="grid grid-cols-1 md:grid-cols-2 gap-6 mb-6">
        <div className="bg-white border border-gray-200 rounded-lg p-4">
          <div className="flex items-center space-x-2 mb-2">
            <Activity className="w-4 h-4 text-gray-600" />
            <span className="text-sm font-medium text-gray-700">Rate Sources</span>
          </div>
          <div className="text-2xl font-bold text-gray-900">
            {rateData.aggregated.sources_count}
          </div>
          <div className="text-sm text-gray-600">
            Active sources contributing to this rate
          </div>
        </div>

        <div className="bg-white border border-gray-200 rounded-lg p-4">
          <div className="flex items-center space-x-2 mb-2">
            <DollarSign className="w-4 h-4 text-gray-600" />
            <span className="text-sm font-medium text-gray-700">Confidence</span>
          </div>
          <div className={`text-2xl font-bold ${getConfidenceColor(rateData.aggregated.deviation_threshold)}`}>
            {getConfidenceLabel(rateData.aggregated.deviation_threshold)}
          </div>
          <div className="text-sm text-gray-600">
            Based on {rateData.aggregated.deviation_threshold}% deviation threshold
          </div>
        </div>
      </div>

      <div className="space-y-4">
        <div className="flex items-center justify-between text-sm">
          <div className="flex items-center space-x-2 text-gray-600">
            <Clock className="w-4 h-4" />
            <span>Last Updated</span>
          </div>
          <span className="text-gray-900">
            {lastUpdated ? format(lastUpdated, 'MMM dd, yyyy HH:mm:ss') : 'Unknown'}
          </span>
        </div>

        <div className="flex items-center justify-between text-sm">
          <span className="text-gray-600">Weighted Average</span>
          <span className="text-gray-900">{formatRate(rateData.aggregated.weighted_average)}</span>
        </div>

        <div className="flex items-center justify-between text-sm">
          <span className="text-gray-600">Deviation Threshold</span>
          <span className="text-gray-900">{rateData.aggregated.deviation_threshold}%</span>
        </div>
      </div>

      {rateData.sources.length > 0 && (
        <div className="mt-6 pt-6 border-t">
          <h3 className="text-lg font-medium text-gray-900 mb-4">Rate Sources</h3>
          <div className="space-y-2">
            {rateData.sources.map((source, index) => (
              <div key={index} className="flex items-center justify-between p-3 bg-gray-50 rounded-lg">
                <div>
                  <div className="font-medium text-gray-900">{source.source}</div>
                  <div className="text-sm text-gray-600">
                    Confidence: {source.confidence}%
                  </div>
                </div>
                <div className="text-right">
                  <div className="font-medium text-gray-900">
                    {formatRate(source.rate)}
                  </div>
                  <div className="text-sm text-gray-600">
                    {format(new Date(source.timestamp * 1000), 'HH:mm:ss')}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      <div className="mt-6 pt-6 border-t">
        <div className="bg-blue-50 border border-blue-200 rounded-md p-4">
          <div className="flex items-start space-x-2">
            <Info className="w-5 h-5 text-blue-600 mt-0.5" />
            <div className="text-sm text-blue-800">
              <p className="font-medium mb-1">About Exchange Rates</p>
              <p className="text-blue-700">
                Exchange rates are aggregated from multiple on-chain sources and updated 
                in real-time. Rates are calculated using a weighted average with confidence 
                scores and deviation filtering to ensure accuracy.
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
