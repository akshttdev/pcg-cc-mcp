import { makeRequest, handleApiResponse, ApiError } from './client';

// ── Workflow Templates ──

export interface WorkflowTaskTemplate {
  title: string;
  description: string;
  position: number;
  task_type: 'agent' | 'human_review' | 'hybrid';
  agent_role?: string;
  requires_approval: boolean;
  priority: string;
  depends_on: number[];
  knowledge_inputs: string[];
  knowledge_outputs: string[];
  tags: string[];
}

export interface WorkflowPhaseTemplate {
  name: string;
  description: string;
  position: number;
  is_recurring: boolean;
  tasks: WorkflowTaskTemplate[];
}

export interface WorkflowTemplate {
  id: string;
  name: string;
  description: string;
  client_type: 'foundation_build' | 'managed_growth' | 'custom';
  is_recurring: boolean;
  phases: WorkflowPhaseTemplate[];
}

export interface DealConversionResult {
  project_id: string;
  project_name: string;
  boards_created: number;
  tasks_created: number;
  dependencies_created: number;
  template_used: string;
}

export interface ConvertDealRequest {
  template_id: string;
  project_name?: string;
  organization_id?: string;
  client_id?: string;
  git_repo_path?: string;
}

export const workflowTemplatesApi = {
  list: async (): Promise<WorkflowTemplate[]> => {
    const response = await makeRequest('/api/workflow-templates');
    return handleApiResponse<WorkflowTemplate[]>(response);
  },

  get: async (id: string): Promise<WorkflowTemplate> => {
    const response = await makeRequest(`/api/workflow-templates/${encodeURIComponent(id)}`);
    return handleApiResponse<WorkflowTemplate>(response);
  },

  convertDeal: async (dealId: string, data: ConvertDealRequest): Promise<DealConversionResult> => {
    const response = await makeRequest(`/api/crm/deals/${dealId}/convert`, {
      method: 'POST',
      body: JSON.stringify(data),
    });
    return handleApiResponse<DealConversionResult>(response);
  },
};

// Aptos Blockchain Types
export interface AptosBalance {
  address: string;
  balance: number;
  balance_apt: number;
  sequence_number: number;
}

export interface AptosTransaction {
  version: string;
  hash: string;
  sender: string;
  sequence_number: string;
  timestamp: string;
  tx_type: string;
  success: boolean;
  gas_used: string;
  gas_unit_price: string;
  payload_function: string | null;
}

export interface FaucetResponse {
  success: boolean;
  message: string;
  tx_hashes: string[];
}

export interface SendTransactionRequest {
  sender_private_key: string;
  sender_address: string;
  recipient_address: string;
  amount_apt: number;
}

export interface SendTransactionResponse {
  success: boolean;
  tx_hash: string;
  message: string;
}

export interface EstimateGasResponse {
  gas_estimate: number;
  gas_unit_price: number;
  total_gas_apt: number;
}

// VIBE Token Types
export interface VibeBalance {
  address: string;
  balance: number;
  balance_vibe: number;
  equivalent_apt: number;
  usd_value: number;
}

export interface SendVibeRequest {
  sender_private_key: string;
  sender_address: string;
  recipient_address: string;
  amount_vibe: number;
}

export interface VibeTransferResponse {
  success: boolean;
  tx_hash: string;
  amount_vibe: number;
  message: string;
}

// Aptos Testnet API
export const aptosApi = {
  getBalance: async (address: string): Promise<AptosBalance> => {
    const response = await makeRequest(`/api/aptos/balance/${encodeURIComponent(address)}`);
    return handleApiResponse<AptosBalance>(response);
  },

  getTransactions: async (address: string, limit?: number): Promise<AptosTransaction[]> => {
    const params = limit ? `?limit=${limit}` : '';
    const response = await makeRequest(`/api/aptos/transactions/${encodeURIComponent(address)}${params}`);
    return handleApiResponse<AptosTransaction[]>(response);
  },

  fundFromFaucet: async (address: string, amount?: number): Promise<FaucetResponse> => {
    const params = amount ? `?amount=${amount}` : '';
    const response = await makeRequest(`/api/aptos/faucet/${encodeURIComponent(address)}${params}`, {
      method: 'POST',
    });
    return handleApiResponse<FaucetResponse>(response);
  },

  accountExists: async (address: string): Promise<boolean> => {
    const response = await makeRequest(`/api/aptos/exists/${encodeURIComponent(address)}`);
    return handleApiResponse<boolean>(response);
  },

  sendApt: async (request: SendTransactionRequest): Promise<SendTransactionResponse> => {
    const response = await makeRequest('/api/aptos/send', {
      method: 'POST',
      body: JSON.stringify(request),
    });
    return handleApiResponse<SendTransactionResponse>(response);
  },

  estimateGas: async (address: string): Promise<EstimateGasResponse> => {
    const response = await makeRequest(`/api/aptos/estimate-gas/${encodeURIComponent(address)}`);
    return handleApiResponse<EstimateGasResponse>(response);
  },

  // VIBE Token Methods
  getVibeBalance: async (address: string): Promise<VibeBalance> => {
    const response = await makeRequest(`/api/vibe/balance/${encodeURIComponent(address)}`);
    return handleApiResponse<VibeBalance>(response);
  },

  sendVibe: async (request: SendVibeRequest): Promise<VibeTransferResponse> => {
    const response = await makeRequest('/api/vibe/send', {
      method: 'POST',
      body: JSON.stringify(request),
    });
    return handleApiResponse<VibeTransferResponse>(response);
  },
};

// ============================================
// VIBE Token Economy API
// ============================================

export interface VibeDepositRecord {
  id: string;
  project_id: string;
  tx_hash: string;
  sender_address: string;
  amount_vibe: number;
  status: string;
  payment_method: string;
  credited_at: string | null;
}

export const vibeApi = {
  getConfig: async (): Promise<{ revenue_address: string; network: string; vibe_token_address: string }> => {
    const response = await makeRequest('/api/vibe/config');
    return handleApiResponse<{ revenue_address: string; network: string; vibe_token_address: string }>(response);
  },

  verifyDeposit: async (projectId: string, txHash: string, amountVibe: number): Promise<VibeDepositRecord> => {
    const response = await makeRequest('/api/vibe/deposit/verify', {
      method: 'POST',
      body: JSON.stringify({ project_id: projectId, tx_hash: txHash, amount_vibe: amountVibe }),
    });
    return handleApiResponse<VibeDepositRecord>(response);
  },
};

// ============================================
// Model Pricing / Billing Rates API
// ============================================

export interface ModelPricing {
  id: string;
  model: string;
  provider: string;
  input_cost_per_million: number;
  output_cost_per_million: number;
  multiplier: number;
  effective_from: string;
  created_at: string;
}

export interface CostEstimate {
  model: string;
  provider: string;
  input_tokens: number;
  output_tokens: number;
  cost_cents: number;
  cost_vibe: number;
  cost_usd: number;
}

export interface UpsertModelPricing {
  model: string;
  provider: string;
  input_cost_per_million: number;
  output_cost_per_million: number;
  multiplier?: number;
}

export const modelPricingApi = {
  list: async (): Promise<ModelPricing[]> => {
    const response = await makeRequest('/api/model-pricing');
    if (!response.ok) {
      throw new ApiError('Failed to load model pricing', response.status, response);
    }
    return response.json();
  },

  get: async (model: string, provider: string): Promise<ModelPricing> => {
    const response = await makeRequest(`/api/model-pricing/${encodeURIComponent(model)}/${encodeURIComponent(provider)}`);
    if (!response.ok) {
      throw new ApiError('Failed to load model pricing', response.status, response);
    }
    return response.json();
  },

  estimate: async (model: string, inputTokens: number, outputTokens: number, provider?: string): Promise<CostEstimate> => {
    const params = new URLSearchParams({
      model,
      input_tokens: inputTokens.toString(),
      output_tokens: outputTokens.toString(),
    });
    if (provider) params.set('provider', provider);
    const response = await makeRequest(`/api/model-pricing/estimate?${params.toString()}`);
    if (!response.ok) {
      throw new ApiError('Failed to estimate cost', response.status, response);
    }
    return response.json();
  },

  upsert: async (data: UpsertModelPricing): Promise<ModelPricing> => {
    const response = await makeRequest('/api/model-pricing', {
      method: 'POST',
      body: JSON.stringify(data),
    });
    if (!response.ok) {
      throw new ApiError('Failed to save model pricing', response.status, response);
    }
    return response.json();
  },

  delete: async (id: string): Promise<void> => {
    const response = await makeRequest(`/api/model-pricing/${id}`, {
      method: 'DELETE',
    });
    if (!response.ok) {
      throw new ApiError('Failed to delete model pricing', response.status, response);
    }
  },
};
