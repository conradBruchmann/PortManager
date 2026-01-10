import { useEffect, useState } from 'react';
import axios from 'axios';
import type { Lease } from './types';
import { Network, Plus, Trash2, RefreshCw } from 'lucide-react';

const API_URL = 'http://localhost:3030';

function App() {
  const [leases, setLeases] = useState<Lease[]>([]);
  const [loading, setLoading] = useState(false);
  const [serviceName, setServiceName] = useState('new-service');

  const fetchLeases = async () => {
    setLoading(true);
    try {
      const response = await axios.get<Lease[]>(`${API_URL}/list`);
      setLeases(response.data);
    } catch (error) {
      console.error('Failed to fetch leases', error);
    } finally {
      setLoading(false);
    }
  };

  const allocatePort = async () => {
    try {
      await axios.post(`${API_URL}/alloc`, {
        service_name: serviceName,
        ttl_seconds: 300,
        tags: ['ui-test']
      });
      fetchLeases();
    } catch (error) {
      console.error('Failed to allocate port', error);
    }
  };

  const releasePort = async (port: number) => {
    try {
      await axios.post(`${API_URL}/release`, { port });
      fetchLeases();
    } catch (error) {
      console.error('Failed to release port', error);
    }
  };

  useEffect(() => {
    fetchLeases();
    const interval = setInterval(fetchLeases, 5000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="min-h-screen bg-gray-50 text-gray-900 p-8">
      <div className="max-w-4xl mx-auto space-y-8">
        {/* Header */}
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-3">
            <div className="p-3 bg-blue-600 rounded-lg shadow-lg">
              <Network className="w-8 h-8 text-white" />
            </div>
            <div>
              <h1 className="text-2xl font-bold tracking-tight">Port Manager</h1>
              <p className="text-gray-500">Centralized Local Port Authority</p>
            </div>
          </div>
          <div className="flex items-center space-x-4">
            <button
              onClick={fetchLeases}
              className="p-2 text-gray-400 hover:text-gray-600 transition-colors"
              title="Refresh"
            >
              <RefreshCw className={`w-5 h-5 ${loading ? 'animate-spin' : ''}`} />
            </button>
          </div>
        </div>

        {/* Allocation Card */}
        <div className="bg-white p-6 rounded-xl shadow-sm border border-gray-100 flex items-center space-x-4">
          <input
            type="text"
            value={serviceName}
            onChange={(e) => setServiceName(e.target.value)}
            className="flex-1 px-4 py-2 border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
            placeholder="Service Name"
          />
          <button
            onClick={allocatePort}
            className="flex items-center space-x-2 px-6 py-2 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-lg transition-colors"
          >
            <Plus className="w-4 h-4" />
            <span>Allocate Port</span>
          </button>
        </div>

        {/* Active Leases Table */}
        <div className="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
          <div className="px-6 py-4 border-b border-gray-100 bg-gray-50/50">
            <h2 className="text-lg font-semibold text-gray-800">Active Allocations ({leases.length})</h2>
          </div>
          <div className="overflow-x-auto">
            <table className="w-full text-left">
              <thead>
                <tr className="border-b border-gray-100 text-sm text-gray-500 uppercase tracking-wider">
                  <th className="px-6 py-4 font-medium">Port</th>
                  <th className="px-6 py-4 font-medium">Service</th>
                  <th className="px-6 py-4 font-medium">Allocated</th>
                  <th className="px-6 py-4 font-medium">Expires In</th>
                  <th className="px-6 py-4 font-medium text-right">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-gray-100">
                {leases.length === 0 ? (
                  <tr>
                    <td colSpan={5} className="px-6 py-8 text-center text-gray-400 italic">
                      No ports currently allocated.
                    </td>
                  </tr>
                ) : (
                  leases.map((lease) => {
                    const allocated = new Date(lease.allocated_at);
                    return (
                      <tr key={lease.port} className="hover:bg-gray-50/50 transition-colors">
                        <td className="px-6 py-4 font-mono font-medium text-blue-600">
                          {lease.port}
                        </td>
                        <td className="px-6 py-4 font-medium text-gray-900">
                          {lease.service_name}
                        </td>
                        <td className="px-6 py-4 text-sm text-gray-500">
                          {allocated.toLocaleString()}
                        </td>
                        <td className="px-6 py-4 text-sm text-gray-500">
                          {lease.ttl_seconds}s
                        </td>
                        <td className="px-6 py-4 text-right">
                          <button
                            onClick={() => releasePort(lease.port)}
                            className="p-2 text-gray-400 hover:text-red-500 hover:bg-red-50 rounded-lg transition-colors"
                            title="Release Port"
                          >
                            <Trash2 className="w-4 h-4" />
                          </button>
                        </td>
                      </tr>
                    );
                  })
                )}
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;
