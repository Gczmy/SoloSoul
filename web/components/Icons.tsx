'use client'

import React from 'react'

interface IconProps {
  size?: number
  className?: string
  color?: string
  strokeWidth?: number
}

export function LockIcon({ size = 20, className = '', color = 'currentColor', strokeWidth = 2 }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={`anim-icon anim-icon-lock ${className}`}
    >
      <rect
        x="3"
        y="11"
        width="18"
        height="11"
        rx="2"
        ry="2"
        className="lock-body"
        fill={color}
        fillOpacity={0.1}
      />
      <path
        d="M7 11V7a5 5 0 0 1 10 0v4"
        className="lock-shackle"
      />
    </svg>
  )
}

export function LockOpenIcon({ size = 20, className = '', color = 'currentColor', strokeWidth = 2 }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={`anim-icon anim-icon-lock ${className}`}
    >
      <rect
        x="3"
        y="11"
        width="18"
        height="11"
        rx="2"
        ry="2"
        className="lock-body"
        fill={color}
        fillOpacity={0.1}
      />
      <path
        d="M7 11V7a5 5 0 0 1 9.9-1"
        className="lock-shackle"
      />
    </svg>
  )
}

export function EyeIcon({ size = 20, className = '', color = 'currentColor', strokeWidth = 2 }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={`anim-icon anim-icon-eye ${className}`}
    >
      <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" className="eye-outer" />
      <circle cx="12" cy="12" r="3" className="eye-pupil" fill={color} fillOpacity={0.3} />
    </svg>
  )
}

export function EyeOffIcon({ size = 20, className = '', color = 'currentColor', strokeWidth = 2 }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={`anim-icon ${className}`}
    >
      <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94" />
      <path d="M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19" />
      <line x1="1" y1="1" x2="23" y2="23" />
    </svg>
  )
}

interface ChevronIconProps extends IconProps {
  direction?: 'right' | 'down' | 'left' | 'up'
  open?: boolean
}

export function ChevronIcon({
  size = 16,
  className = '',
  color = 'currentColor',
  strokeWidth = 2,
  direction = 'right',
  open = false
}: ChevronIconProps) {
  const rotation = {
    right: 0,
    down: 90,
    left: 180,
    up: 270
  }[direction]

  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={`anim-icon anim-icon-chevron ${open ? 'open' : ''} ${className}`}
      style={{ transform: open ? 'rotate(90deg)' : `rotate(${rotation}deg)` }}
    >
      <polyline points="9 18 15 12 9 6" />
    </svg>
  )
}

export function WarningIcon({ size = 36, className = '', color = '#fbbf24', strokeWidth = 2.5 }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={`anim-icon ${className}`}
      style={{ filter: `drop-shadow(0 0 12px ${color}60)` }}
    >
      {/* Triangle with visible fill */}
      <path
        d="M12 2L22 20H2L12 2Z"
        fill={color}
        fillOpacity={0.25}
      />
      {/* Exclamation mark - bold */}
      <line x1="12" y1="9" x2="12" y2="14" strokeLinecap="round" />
      <circle cx="12" cy="17.5" r="1.5" fill={color} stroke="none" />
    </svg>
  )
}

export function ShieldIcon({ size = 20, className = '', color = 'currentColor', strokeWidth = 2 }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={`anim-icon anim-icon-lift ${className}`}
    >
      <path
        d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"
        fill={color}
        fillOpacity={0.1}
      />
      <path d="M9 12l2 2 4-4" />
    </svg>
  )
}

export function KeyIcon({ size = 20, className = '', color = 'currentColor', strokeWidth = 2 }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={`anim-icon anim-icon-lift ${className}`}
    >
      <circle cx="8" cy="8" r="5" fill={color} fillOpacity={0.1} />
      <path d="M11.3 11.3L21 21" />
      <path d="M16 16l2 2" />
      <path d="M19 13l2 2" />
    </svg>
  )
}

export function UserIcon({ size = 20, className = '', color = 'currentColor', strokeWidth = 2 }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={`anim-icon ${className}`}
    >
      <circle cx="12" cy="8" r="4" fill={color} fillOpacity={0.1} />
      <path d="M20 21a8 8 0 1 0-16 0" />
    </svg>
  )
}

export function HomeIcon({ size = 20, className = '', color = 'currentColor', strokeWidth = 2 }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={`anim-icon anim-icon-lift ${className}`}
    >
      <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" fill={color} fillOpacity={0.1} />
      <polyline points="9 22 9 12 15 12 15 22" />
    </svg>
  )
}

export function FolderIcon({ size = 20, className = '', color = 'currentColor', strokeWidth = 2 }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={`anim-icon anim-icon-lift ${className}`}
    >
      <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" fill={color} fillOpacity={0.1} />
    </svg>
  )
}

export function ScanIcon({ size = 20, className = '', color = 'currentColor', strokeWidth = 2 }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={`anim-icon ${className}`}
    >
      <rect x="3" y="3" width="18" height="18" rx="2" fill={color} fillOpacity={0.1} />
      <line x1="3" y1="9" x2="21" y2="9" className="anim-icon-scan-line" />
      <line x1="3" y1="15" x2="21" y2="15" className="anim-icon-scan-line" style={{ animationDelay: '0.5s' }} />
      <line x1="9" y1="3" x2="9" y2="21" />
      <line x1="15" y1="3" x2="15" y2="21" />
    </svg>
  )
}

export function PlugIcon({ size = 20, className = '', color = 'currentColor', strokeWidth = 2 }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={`anim-icon anim-icon-lift ${className}`}
    >
      <path d="M12 22v-5" />
      <path d="M9 7V2" />
      <path d="M15 7V2" />
      <rect x="5" y="7" width="14" height="5" rx="1" fill={color} fillOpacity={0.1} />
      <path d="M5 12h14" />
    </svg>
  )
}

export function SettingsIcon({ size = 20, className = '', color = 'currentColor', strokeWidth = 2 }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={`anim-icon anim-icon-lift ${className}`}
    >
      <circle cx="12" cy="12" r="3" fill={color} fillOpacity={0.1} />
      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
    </svg>
  )
}

export function SpinnerIcon({ size = 20, className = '', color = 'currentColor', strokeWidth = 2 }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={`anim-icon anim-icon-spin ${className}`}
    >
      <path d="M21 12a9 9 0 1 1-6.219-8.56" />
    </svg>
  )
}

export function CheckIcon({ size = 20, className = '', color = 'currentColor', strokeWidth = 2.5 }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={`anim-icon anim-icon-check ${className}`}
    >
      <polyline points="20 6 9 17 4 12" />
    </svg>
  )
}

export function TrashIcon({ size = 20, className = '', color = 'currentColor', strokeWidth = 2 }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={`anim-icon anim-icon-lift anim-icon-press ${className}`}
    >
      <path d="M3 6h18" />
      <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6" />
      <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" fill={color} fillOpacity={0.1} />
      <line x1="10" y1="11" x2="10" y2="17" />
      <line x1="14" y1="11" x2="14" y2="17" />
    </svg>
  )
}

export function DownloadIcon({ size = 20, className = '', color = 'currentColor', strokeWidth = 2 }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={`anim-icon ${className}`}
    >
      <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
      <polyline points="7 10 12 15 17 10" />
      <line x1="12" y1="15" x2="12" y2="3" />
    </svg>
  )
}

export function PlusIcon({ size = 20, className = '', color = 'currentColor', strokeWidth = 2 }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={`anim-icon ${className}`}
    >
      <line x1="12" y1="5" x2="12" y2="19" />
      <line x1="5" y1="12" x2="19" y2="12" />
    </svg>
  )
}

export function ArrowLeftIcon({ size = 20, className = '', color = 'currentColor', strokeWidth = 2 }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={`anim-icon ${className}`}
    >
      <line x1="19" y1="12" x2="5" y2="12" />
      <polyline points="12 19 5 12 12 5" />
    </svg>
  )
}

export function VaultIcon({ size = 20, className = '', color = 'currentColor', strokeWidth = 1.5 }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={`anim-icon anim-icon-lift ${className}`}
    >
      <rect x="3" y="3" width="18" height="18" rx="3" fill={color} fillOpacity={0.08} />
      <circle cx="12" cy="12" r="4" fill={color} fillOpacity={0.15} />
      <circle cx="12" cy="12" r="1.5" fill={color} />
      <path d="M12 8v-2" />
      <path d="M12 18v-2" />
      <path d="M8 12H6" />
      <path d="M18 12h-2" />
    </svg>
  )
}

export function ProfileIcon({ size = 20, className = '', color = 'currentColor', strokeWidth = 1.5 }: IconProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={`anim-icon anim-icon-lift ${className}`}
    >
      <circle cx="12" cy="8" r="3.5" fill={color} fillOpacity={0.1} />
      <path d="M4 21v-1a4 4 0 0 1 4-4h8a4 4 0 0 1 4 4v1" />
      <path d="M12 14l2-2 2 2" strokeLinecap="round" />
    </svg>
  )
}
