import { NextRequest, NextResponse } from 'next/server'

const API_BASE = process.env.SOLOSOUL_API_URL || 'http://localhost:8080'

export async function POST(req: NextRequest) {
  try {
    const body = await req.json()
    const { account_id, master_password } = body

    if (!master_password || master_password.length < 8) {
      return NextResponse.json(
        { success: false, error: 'Password must be at least 8 characters' },
        { status: 400 }
      )
    }

    if (!account_id) {
      return NextResponse.json(
        { success: false, error: 'account_id is required' },
        { status: 400 }
      )
    }

    // Forward to Go backend
    const res = await fetch(`${API_BASE}/api/auth/unlock`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ account_id, master_password }),
    })

    const data = await res.json()
    return NextResponse.json(data, { status: res.status })
  } catch {
    return NextResponse.json({ success: false, error: 'Invalid request' }, { status: 400 })
  }
}
