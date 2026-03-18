import { Clock, Users, ListTodo, Activity } from 'lucide-react';

export default function StatCard({ title, value, icon, color, iconBg }) {
  const Icon = icon;
  return (
    <div className="bg-white rounded-2xl shadow-sm p-6 border border-gray-100 hover:shadow-md transition-shadow">
      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm text-gray-500 mb-1">{title}</p>
          <p className="text-3xl font-bold text-gray-800">{value}</p>
        </div>
        <div className={`w-14 h-14 rounded-2xl flex items-center justify-center ${iconBg}`}>
          <Icon className={`w-7 h-7 ${color}`} />
        </div>
      </div>
    </div>
  );
}
