import { NextRequest, NextResponse } from 'next/server'

// Mock auth status - in production, this would call the Go backend
export async function GET() {
  return NextResponse.json({
    initialized: true,
    locked: true,
    profiles: [],
  })
}
