import { useState } from 'react';
import { GoogleOAuthProvider, GoogleLogin } from '@react-oauth/google';
import { api } from './api';

const GOOGLE_CLIENT_ID = import.meta.env.VITE_GOOGLE_CLIENT_ID || '';

function LoginForm({ onLogin }) {
  const [error, setError] = useState(null);
  const [loading, setLoading] = useState(false);

  const handleGoogleSuccess = async (credentialResponse) => {
    setLoading(true);
    setError(null);
    try {
      console.log('Sending credential to backend...');
      const response = await api.login(credentialResponse.credential);
      console.log('Login response:', response);
      localStorage.setItem('token', response.access_token);
      localStorage.setItem('user', JSON.stringify(response.user));
      onLogin(response.user);
    } catch (err) {
      console.error('Login error:', err);
      setError(`Login failed: ${err.message}`);
    } finally {
      setLoading(false);
    }
  };

  const handleGoogleError = (err) => {
    console.error('Google error:', err);
    setError('Google sign-in failed. Check console for details.');
  };

  return (
    <div className="min-h-screen bg-gray-100 flex items-center justify-center p-4">
      <div className="bg-white rounded-2xl shadow-lg p-8 w-full max-w-md">
        <h1 className="text-3xl font-bold text-gray-800 mb-2 text-center">Time Tracker</h1>
        <p className="text-gray-500 text-center mb-6">Sign in to access your dashboard</p>
        {error && <div className="mb-4 p-3 bg-red-50 text-red-600 rounded-lg">{error}</div>}
        <div className="flex justify-center">
          <GoogleLogin onSuccess={handleGoogleSuccess} onError={handleGoogleError} useOneTap />
        </div>
        {loading && <p className="mt-4 text-center text-gray-500">Signing in...</p>}
      </div>
    </div>
  );
}

export default function Login({ onLogin }) {
  return (
    <GoogleOAuthProvider clientId={GOOGLE_CLIENT_ID}>
      <LoginForm onLogin={onLogin} />
    </GoogleOAuthProvider>
  );
}
