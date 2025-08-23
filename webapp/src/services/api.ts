import type {
  HealthResponse,
  HelloResponse,
  ApiError,
} from '../types/generated';

const API_BASE_URL =
  import.meta.env.VITE_API_URL || 'http://localhost:8080/api';

export interface ApiResponse<T> {
  data?: T;
  error?: string;
}

// Re-export generated types for convenience
export type { HealthResponse, HelloResponse, ApiError };

class ApiService {
  private async request<T>(endpoint: string): Promise<ApiResponse<T>> {
    try {
      const response = await fetch(`${API_BASE_URL}${endpoint}`);

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const data = await response.json();
      return { data };
    } catch (error) {
      console.error('API request failed:', error);
      return {
        error: error instanceof Error ? error.message : 'Unknown error',
      };
    }
  }

  async healthCheck(): Promise<ApiResponse<HealthResponse>> {
    return this.request<HealthResponse>('/health');
  }

  async getHello(): Promise<ApiResponse<HelloResponse>> {
    return this.request<HelloResponse>('/hello');
  }
}

export const apiService = new ApiService();
