import React, { useState, useRef, useCallback, useEffect } from 'react';
import './App.css';

// ─── Types ────────────────────────────────────────────────────────────────────

type ServerMsg =
  | { type: 'chat'; conversation: string; from: string; message: string }
  | { type: 'info'; message: string }
  | { type: 'error'; message: string };

type LogEntry = {
  id: number;
  direction: 'sent' | 'received';
  timestamp: string;
  content: string;
};

type ChatMessage = {
  id: number;
  kind: 'chat' | 'info' | 'error' | 'system';
  from?: string;
  content: string;
  timestamp: string;
};

type WsStatus = 'disconnected' | 'connecting' | 'connected';

// ─── Helpers ──────────────────────────────────────────────────────────────────

function now(): string {
  return new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' });
}

function prettyJson(raw: string): string {
  try {
    return JSON.stringify(JSON.parse(raw), null, 2);
  } catch {
    return raw;
  }
}

// ─── App ──────────────────────────────────────────────────────────────────────

function App() {
  // ── Config
  const [authUrl, setAuthUrl] = useState('http://localhost:3000');
  const [socketUrl, setSocketUrl] = useState('ws://localhost:9901');

  // ── Auth state
  const [token, setToken] = useState('');
  const [userId, setUserId] = useState('');
  const [loggedInAs, setLoggedInAs] = useState('');

  // ── Register form
  const [regUser, setRegUser] = useState('');
  const [regPass, setRegPass] = useState('');

  // ── Login form
  const [loginUser, setLoginUser] = useState('');
  const [loginPass, setLoginPass] = useState('');

  // ── WebSocket
  const [wsStatus, setWsStatus] = useState<WsStatus>('disconnected');
  const wsRef = useRef<WebSocket | null>(null);

  // ── Conversation
  const [participantId, setParticipantId] = useState('');
  const [conversationId, setConversationId] = useState('');

  // ── Messages + log
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [log, setLog] = useState<LogEntry[]>([]);
  const [msgInput, setMsgInput] = useState('');

  const msgIdRef = useRef(0);
  const logIdRef = useRef(0);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  // Scroll to bottom when new messages arrive
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  const addMsg = useCallback((msg: Omit<ChatMessage, 'id' | 'timestamp'>) => {
    setMessages(prev => [...prev, { ...msg, id: ++msgIdRef.current, timestamp: now() }]);
  }, []);

  const addLog = useCallback((direction: 'sent' | 'received', content: string) => {
    setLog(prev => [...prev, { id: ++logIdRef.current, direction, timestamp: now(), content }]);
  }, []);

  // ── WS message handler ref (avoids stale closures in WS callbacks)
  // Assigned every render so it always captures the latest state.
  const handleWsMessageRef = useRef<(event: MessageEvent) => void>(() => {});
  handleWsMessageRef.current = (event: MessageEvent) => {
    addLog('received', event.data);
    try {
      const msg: ServerMsg = JSON.parse(event.data);
      if (msg.type === 'chat') {
        addMsg({ kind: 'chat', from: msg.from, content: msg.message });
        // Auto-fill conversation ID on first incoming chat message
        setConversationId(prev => prev || msg.conversation);
      } else if (msg.type === 'info') {
        addMsg({ kind: 'info', content: msg.message });
        // Auto-detect conversation UUID from create_conversation response
        // Skip AUTH OK messages — those contain a user ID, not a conversation ID
        if (!msg.message.includes('AUTH OK')) {
          const match = msg.message.match(
            /[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i
          );
          if (match) {
            setConversationId(match[0]);
          }
        }
      } else if (msg.type === 'error') {
        addMsg({ kind: 'error', content: msg.message });
      }
    } catch {
      addMsg({ kind: 'error', content: `[unparseable] ${event.data}` });
    }
  };

  // ── Auth handlers ─────────────────────────────────────────────────────────

  const handleRegister = async () => {
    try {
      const res = await fetch(`${authUrl}/register`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ username: regUser, password: regPass }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.message ?? res.statusText);
      addMsg({ kind: 'system', content: `Registered: ${data.username} (${data.id})` });
    } catch (e: any) {
      addMsg({ kind: 'error', content: `Register failed: ${e.message}` });
    }
  };

  const handleLogin = async () => {
    try {
      const res = await fetch(`${authUrl}/login`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ username: loginUser, password: loginPass }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.message ?? res.statusText);
      setToken(data.token);
      setUserId(data.id);
      setLoggedInAs(data.username);
      addMsg({ kind: 'system', content: `Logged in as ${data.username} (${data.id})` });
    } catch (e: any) {
      addMsg({ kind: 'error', content: `Login failed: ${e.message}` });
    }
  };

  // ── WebSocket handlers ────────────────────────────────────────────────────

  const handleConnect = () => {
    if (wsRef.current) return;
    setWsStatus('connecting');
    const ws = new WebSocket(socketUrl);
    wsRef.current = ws;

    ws.onopen = () => {
      setWsStatus('connected');
      addMsg({ kind: 'system', content: `Connected to ${socketUrl}` });
    };

    ws.onmessage = (event) => handleWsMessageRef.current(event);

    ws.onerror = () => {
      addMsg({ kind: 'error', content: 'WebSocket error' });
    };

    ws.onclose = () => {
      wsRef.current = null;
      setWsStatus('disconnected');
      addMsg({ kind: 'system', content: 'WebSocket disconnected' });
    };
  };

  const handleDisconnect = () => {
    wsRef.current?.close();
  };

  const sendCmd = useCallback((cmd: object) => {
    if (!wsRef.current || wsRef.current.readyState !== WebSocket.OPEN) {
      addMsg({ kind: 'error', content: 'Not connected' });
      return;
    }
    const raw = JSON.stringify(cmd);
    addLog('sent', raw);
    wsRef.current.send(raw);
  }, [addMsg, addLog]);

  const handleAuthenticate = () => sendCmd({ type: 'authenticate', token });

  const handleCreateConversation = () =>
    sendCmd({ type: 'create_conversation', participant: participantId });

  const handleSendMessage = (e: React.SyntheticEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (!msgInput.trim()) return;
    sendCmd({ type: 'say', message: msgInput, conversation_id: conversationId });
    setMsgInput('');
  };

  // ── Status helpers ────────────────────────────────────────────────────────

  const isConnected = wsStatus === 'connected';

  const statusColors: Record<WsStatus, string> = {
    disconnected: 'var(--red)',
    connecting: 'var(--amber)',
    connected: 'var(--green)',
  };

  return (
    <div className="app">

      {/* ── Config bar ──────────────────────────────────────────────────────── */}
      <header className="config-bar">
        <span className="app-title">Chat Tester</span>
        <label className="config-field">
          Auth URL
          <input value={authUrl} onChange={e => setAuthUrl(e.target.value)} />
        </label>
        <label className="config-field">
          Socket URL
          <input value={socketUrl} onChange={e => setSocketUrl(e.target.value)} />
        </label>
        <span className="ws-badge" style={{ background: statusColors[wsStatus] }}>
          {wsStatus}
        </span>
      </header>

      <div className="layout">

        {/* ── Left sidebar ─────────────────────────────────────────────────── */}
        <aside className="sidebar">

          {/* Auth panel */}
          <section className="panel">
            <h3 className="panel-title">Auth Service</h3>

            {loggedInAs && (
              <div className="pill pill-green">Logged in as <strong>{loggedInAs}</strong></div>
            )}
            {token && (
              <div className="token-preview" title={token}>
                {token.slice(0, 32)}…
              </div>
            )}

            <fieldset>
              <legend>Register</legend>
              <input
                placeholder="Username"
                value={regUser}
                onChange={e => setRegUser(e.target.value)}
              />
              <input
                type="password"
                placeholder="Password"
                value={regPass}
                onChange={e => setRegPass(e.target.value)}
              />
              <button onClick={handleRegister}>Register</button>
            </fieldset>

            <fieldset>
              <legend>Login</legend>
              <input
                placeholder="Username"
                value={loginUser}
                onChange={e => setLoginUser(e.target.value)}
              />
              <input
                type="password"
                placeholder="Password"
                value={loginPass}
                onChange={e => setLoginPass(e.target.value)}
              />
              <button onClick={handleLogin}>Login</button>
            </fieldset>
          </section>

          {/* WebSocket panel */}
          <section className="panel">
            <h3 className="panel-title">WebSocket</h3>
            <div className="row">
              <button
                onClick={handleConnect}
                disabled={wsStatus !== 'disconnected'}
                className="btn-primary"
              >
                Connect
              </button>
              <button
                onClick={handleDisconnect}
                disabled={wsStatus === 'disconnected'}
                className="btn-danger"
              >
                Disconnect
              </button>
            </div>
            <button
              onClick={handleAuthenticate}
              disabled={!token || !isConnected}
              className="btn-full"
            >
              Send JWT →
            </button>
          </section>

          {/* Conversation panel */}
          <section className="panel">
            <h3 className="panel-title">Conversation</h3>

            <label className="field-label">Participant User ID</label>
            <input
              placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
              value={participantId}
              onChange={e => setParticipantId(e.target.value)}
            />
            <button
              onClick={handleCreateConversation}
              disabled={!participantId || !isConnected}
              className="btn-full"
            >
              Create Conversation
            </button>

            <label className="field-label">
              Conversation ID
              <span className="field-hint">(auto-filled or paste manually)</span>
            </label>
            <input
              placeholder="auto-filled after create"
              value={conversationId}
              onChange={e => setConversationId(e.target.value)}
            />

            {userId && (
              <CopyField label="Your User ID" value={userId} />
            )}
          </section>

        </aside>

        {/* ── Chat area ────────────────────────────────────────────────────── */}
        <main className="chat-area">
          <div className="messages">
            {messages.length === 0 && (
              <div className="empty-state">
                Register or login, connect to the socket server, then start chatting.
              </div>
            )}
            {messages.map(m => (
              <div key={m.id} className={`msg msg-${m.kind}`}>
                <span className="msg-time">{m.timestamp}</span>
                {m.from && (
                  <span className="msg-from" title={m.from}>
                    {m.from.slice(0, 8)}…
                  </span>
                )}
                <span className="msg-body">{m.content}</span>
              </div>
            ))}
            <div ref={messagesEndRef} />
          </div>

          <form className="send-bar" onSubmit={handleSendMessage}>
            <input
              value={msgInput}
              onChange={e => setMsgInput(e.target.value)}
              placeholder={
                !isConnected
                  ? 'Connect to the socket server first'
                  : !conversationId
                  ? 'Create a conversation first'
                  : 'Type a message…'
              }
              disabled={!isConnected || !conversationId}
            />
            <button
              type="submit"
              disabled={!isConnected || !conversationId || !msgInput.trim()}
            >
              Send
            </button>
          </form>
        </main>

        {/* ── Protocol log ─────────────────────────────────────────────────── */}
        <aside className="log-panel">
          <div className="log-header">
            <span className="panel-title">Protocol Log</span>
            <button onClick={() => setLog([])} className="btn-small">Clear</button>
          </div>
          <div className="log-entries">
            {log.length === 0 && (
              <div className="log-empty">No frames yet</div>
            )}
            {log.map(entry => (
              <div key={entry.id} className={`log-entry log-${entry.direction}`}>
                <div className="log-meta">
                  <span className="log-arrow">{entry.direction === 'sent' ? '↑ sent' : '↓ recv'}</span>
                  <span className="log-time">{entry.timestamp}</span>
                </div>
                <pre className="log-body">{prettyJson(entry.content)}</pre>
              </div>
            ))}
          </div>
        </aside>

      </div>
    </div>
  );
}

// ─── CopyField ────────────────────────────────────────────────────────────────

function CopyField({ label, value }: { label: string; value: string }) {
  const [copied, setCopied] = useState(false);

  const handleCopy = () => {
    navigator.clipboard.writeText(value).then(() => {
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    });
  };

  return (
    <div className="copy-field">
      <span className="field-label">{label}</span>
      <button className="copy-value" onClick={handleCopy} title="Click to copy">
        <code>{value.slice(0, 18)}…</code>
        <span className="copy-hint">{copied ? 'copied!' : 'copy'}</span>
      </button>
    </div>
  );
}

export default App;
