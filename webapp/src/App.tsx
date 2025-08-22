import { createSignal, onMount } from 'solid-js';
import { apiService, type HelloResponse, type HealthResponse } from './services/api';

function App() {
  const [healthStatus, setHealthStatus] = createSignal<HealthResponse | null>(null);
  const [helloMessage, setHelloMessage] = createSignal<HelloResponse | null>(null);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  onMount(async () => {
    setLoading(true);
    setError(null);

    // Test health endpoint
    const healthResult = await apiService.healthCheck();
    if (healthResult.error) {
      setError(`Health check failed: ${healthResult.error}`);
    } else if (healthResult.data) {
      setHealthStatus(healthResult.data);
    }

    // Test hello endpoint  
    const helloResult = await apiService.getHello();
    if (helloResult.error) {
      setError(`Hello API failed: ${helloResult.error}`);
    } else if (helloResult.data) {
      setHelloMessage(helloResult.data);
    }

    setLoading(false);
  });

  return (
    <div class="min-h-screen bg-gray-100 py-8">
      <div class="max-w-4xl mx-auto px-4">
        <header class="text-center mb-8">
          <h1 class="text-4xl font-bold text-gray-900 mb-2">Amplify</h1>
          <p class="text-lg text-gray-600">Help founders amplify their products</p>
        </header>

        <div class="bg-white rounded-lg shadow-md p-6 mb-6">
          <h2 class="text-2xl font-semibold mb-4">Backend API Integration Test</h2>
          
          {loading() && (
            <div class="text-center py-4">
              <div class="inline-block animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
              <p class="mt-2 text-gray-600">Testing API connection...</p>
            </div>
          )}

          {error() && (
            <div class="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4">
              <p><strong>Error:</strong> {error()}</p>
              <p class="text-sm mt-1">Make sure the backend is running on http://localhost:8080</p>
            </div>
          )}

          {!loading() && !error() && (
            <div class="space-y-4">
              {healthStatus() && (
                <div class="bg-green-100 border border-green-400 text-green-700 px-4 py-3 rounded">
                  <h3 class="font-semibold">✅ Health Check Success</h3>
                  <p>Status: {healthStatus()!.status}</p>
                  <p>Timestamp: {healthStatus()!.timestamp}</p>
                </div>
              )}

              {helloMessage() && (
                <div class="bg-blue-100 border border-blue-400 text-blue-700 px-4 py-3 rounded">
                  <h3 class="font-semibold">✅ Hello API Success</h3>
                  <p>Message: {helloMessage()!.message}</p>
                  <p>Service: {helloMessage()!.service}</p>
                  <p>Version: {helloMessage()!.version}</p>
                </div>
              )}
            </div>
          )}
        </div>

        <footer class="text-center text-gray-500 text-sm">
          <p>Frontend: SolidJS + TypeScript + Tailwind CSS</p>
          <p>Backend: Rust + Actix Web</p>
        </footer>
      </div>
    </div>
  );
}

export default App;