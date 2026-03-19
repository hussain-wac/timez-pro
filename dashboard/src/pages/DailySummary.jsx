import { useState, useEffect } from 'react';
import { Calendar, Clock, TrendingUp, ChevronLeft } from 'lucide-react';
import { useParams, useNavigate, useSearchParams } from 'react-router-dom';
import { dashboardApi } from '../api';
import { formatHours, formatDuration } from '../utils/format';

export default function DailySummary() {
  const { userId } = useParams();
  const navigate = useNavigate();
  const [searchParams] = useSearchParams();
  const [summary, setSummary] = useState(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(null);
  const [selectedDate, setSelectedDate] = useState(() => {
    const dateParam = searchParams.get('date');
    if (dateParam) return dateParam;
    const today = new Date();
    return today.toISOString().split('T')[0];
  });

  const fetchSummary = async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await dashboardApi.getUserDailySummary(userId, selectedDate);
      setSummary(data);
    } catch (err) {
      console.error('Failed to fetch daily summary:', err);
      setError('Failed to load daily summary');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchSummary();
  }, [userId, selectedDate]);

  const targetSeconds = 8 * 3600; // 8 hours in seconds
  const progressPercent = summary
    ? Math.min((summary.total_seconds / targetSeconds) * 100, 100)
    : 0;

  const formatDate = (dateStr) => {
    const date = new Date(dateStr + 'T00:00:00');
    return date.toLocaleDateString('en-US', {
      weekday: 'long',
      year: 'numeric',
      month: 'long',
      day: 'numeric'
    });
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-64">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-blue-600"></div>
      </div>
    );
  }

  if (error || !summary) {
    return (
      <div className="space-y-6">
        <button
          onClick={() => navigate(-1)}
          className="flex items-center gap-2 text-gray-600 hover:text-gray-800 transition-colors"
        >
          <ChevronLeft className="w-5 h-5" />
          <span>Back</span>
        </button>

        <div className="bg-white rounded-2xl p-12 text-center border border-gray-100">
          <p className="text-red-500">{error || 'Failed to load data'}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <button
            onClick={() => navigate(-1)}
            className="flex items-center justify-center w-10 h-10 rounded-lg bg-white border border-gray-200 hover:bg-gray-50 transition-colors"
          >
            <ChevronLeft className="w-5 h-5 text-gray-600" />
          </button>

          <div>
            <h1 className="text-2xl font-bold text-gray-800">Daily Work Summary</h1>
            <p className="text-gray-500 mt-1">
              {summary.user?.name || 'Employee'} - {formatDate(summary.date)}
            </p>
          </div>
        </div>

        <div className="flex items-center gap-2 bg-white rounded-xl p-2 border border-gray-200">
          <Calendar className="w-5 h-5 text-gray-400" />
          <input
            type="date"
            value={selectedDate}
            onChange={(e) => setSelectedDate(e.target.value)}
            max={new Date().toISOString().split('T')[0]}
            className="px-3 py-2 bg-transparent border-none outline-none cursor-pointer text-gray-700 font-medium"
          />
        </div>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div className="bg-gradient-to-br from-blue-500 to-blue-600 rounded-2xl p-6 text-white shadow-lg">
          <div className="flex items-center justify-between mb-2">
            <span className="text-blue-100 text-sm font-medium">Total Work Time</span>
            <Clock className="w-5 h-5 text-blue-200" />
          </div>
          <p className="text-3xl font-bold">{formatHours(summary.total_seconds)}</p>
          <p className="text-blue-100 text-sm mt-1">{formatDuration(summary.total_seconds)}</p>
        </div>

        <div className="bg-white rounded-2xl p-6 border border-gray-200 shadow-sm">
          <div className="flex items-center justify-between mb-2">
            <span className="text-gray-500 text-sm font-medium">Target Hours</span>
            <TrendingUp className="w-5 h-5 text-gray-400" />
          </div>
          <p className="text-3xl font-bold text-gray-800">8h 0m</p>
          <p className="text-gray-500 text-sm mt-1">Daily goal</p>
        </div>

        <div className="bg-white rounded-2xl p-6 border border-gray-200 shadow-sm">
          <div className="flex items-center justify-between mb-2">
            <span className="text-gray-500 text-sm font-medium">Tasks Worked</span>
            <div className="w-8 h-8 rounded-lg bg-purple-100 flex items-center justify-center">
              <span className="text-purple-600 font-bold">{summary.tasks?.length || 0}</span>
            </div>
          </div>
          <p className="text-3xl font-bold text-gray-800">{summary.tasks?.length || 0}</p>
          <p className="text-gray-500 text-sm mt-1">
            {summary.tasks?.length === 1 ? 'task' : 'tasks'} completed
          </p>
        </div>
      </div>

      <div className="bg-white rounded-2xl p-6 border border-gray-200 shadow-sm">
        <div className="mb-4">
          <div className="flex items-center justify-between mb-2">
            <span className="text-sm font-medium text-gray-700">Daily Progress</span>
            <span className="text-sm font-bold text-gray-800">{progressPercent.toFixed(0)}%</span>
          </div>
          <div className="w-full h-3 bg-gray-100 rounded-full overflow-hidden">
            <div
              className={`h-full transition-all duration-500 rounded-full ${
                progressPercent >= 100
                  ? 'bg-gradient-to-r from-green-400 to-green-500'
                  : progressPercent >= 75
                  ? 'bg-gradient-to-r from-blue-400 to-blue-500'
                  : progressPercent >= 50
                  ? 'bg-gradient-to-r from-yellow-400 to-yellow-500'
                  : 'bg-gradient-to-r from-orange-400 to-orange-500'
              }`}
              style={{ width: `${progressPercent}%` }}
            />
          </div>
          <p className="text-xs text-gray-500 mt-2">
            {summary.total_seconds >= targetSeconds
              ? `Great! You've reached your daily goal.`
              : `${formatHours(targetSeconds - summary.total_seconds)} remaining to reach 8 hours`
            }
          </p>
        </div>
      </div>

      <div className="bg-white rounded-2xl border border-gray-100 shadow-sm overflow-hidden">
        <div className="p-6 border-b border-gray-100">
          <h2 className="text-lg font-semibold text-gray-800">Tasks Breakdown</h2>
          <p className="text-sm text-gray-500 mt-1">Time spent on each task</p>
        </div>

        {summary.tasks && summary.tasks.length > 0 ? (
          <div className="divide-y divide-gray-100">
            {summary.tasks.map((task, index) => {
              const taskPercent = (task.total_seconds / summary.total_seconds) * 100;
              return (
                <div key={index} className="p-4 hover:bg-gray-50 transition-colors">
                  <div className="flex items-center justify-between mb-2">
                    <h3 className="font-medium text-gray-800">{task.task_name}</h3>
                    <span className="text-sm font-semibold text-blue-600">
                      {formatHours(task.total_seconds)}
                    </span>
                  </div>

                  <div className="flex items-center gap-3">
                    <div className="flex-1 h-2 bg-gray-100 rounded-full overflow-hidden">
                      <div
                        className="h-full bg-gradient-to-r from-blue-400 to-purple-500 rounded-full transition-all duration-500"
                        style={{ width: `${taskPercent}%` }}
                      />
                    </div>
                    <span className="text-xs font-medium text-gray-500 min-w-[3rem] text-right">
                      {taskPercent.toFixed(0)}%
                    </span>
                  </div>

                  <p className="text-xs text-gray-500 mt-2">{formatDuration(task.total_seconds)}</p>
                </div>
              );
            })}
          </div>
        ) : (
          <div className="p-12 text-center text-gray-500">
            <Clock className="w-12 h-12 text-gray-300 mx-auto mb-4" />
            <p>No work recorded for this day</p>
          </div>
        )}
      </div>
    </div>
  );
}
