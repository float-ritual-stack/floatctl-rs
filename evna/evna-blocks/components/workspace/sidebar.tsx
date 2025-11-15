/**
 * Workspace Sidebar Component
 *
 * Navigation and quick links for the workspace.
 */

'use client';

import { Home, Search, Clock, Settings } from 'lucide-react';

export function WorkspaceSidebar() {
  return (
    <div className="p-4">
      <nav className="space-y-1">
        <SidebarItem icon={<Home className="w-4 h-4" />} label="Today" active />
        <SidebarItem icon={<Clock className="w-4 h-4" />} label="Recent" />
        <SidebarItem icon={<Search className="w-4 h-4" />} label="Search" />
        <SidebarItem icon={<Settings className="w-4 h-4" />} label="Settings" />
      </nav>

      <div className="mt-8">
        <h3 className="text-xs font-semibold text-gray-500 dark:text-gray-400 uppercase tracking-wider mb-2">
          Recent Sessions
        </h3>
        <div className="space-y-1 text-sm text-gray-600 dark:text-gray-400">
          <div className="hover:text-gray-900 dark:hover:text-gray-100 cursor-pointer">
            Morning brain boot
          </div>
          <div className="hover:text-gray-900 dark:hover:text-gray-100 cursor-pointer">
            floatctl optimization
          </div>
          <div className="hover:text-gray-900 dark:hover:text-gray-100 cursor-pointer">
            BBS design review
          </div>
        </div>
      </div>
    </div>
  );
}

interface SidebarItemProps {
  icon: React.ReactNode;
  label: string;
  active?: boolean;
}

function SidebarItem({ icon, label, active }: SidebarItemProps) {
  return (
    <button
      className={`w-full flex items-center gap-3 px-3 py-2 rounded-md transition-colors ${
        active
          ? 'bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400'
          : 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-700'
      }`}
    >
      {icon}
      <span className="font-medium">{label}</span>
    </button>
  );
}
