export default function StatCard({ title, value, icon, color, iconBg }) {
  const Icon = icon;
  return (
    <div className="bg-white rounded-md border border-gray-200 p-6">
      <div className="flex items-center justify-between">
        <div>
          <p className="text-sm text-gray-600 mb-1">{title}</p>
          <p className="text-2xl font-semibold text-gray-900">{value}</p>
        </div>
        <div className={`w-10 h-10 rounded-md flex items-center justify-center ${iconBg}`}>
          <Icon className={`w-5 h-5 ${color}`} />
        </div>
      </div>
    </div>
  );
}
