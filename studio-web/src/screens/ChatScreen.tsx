import { useState } from 'react';

interface Message {
  role: 'user' | 'assistant';
  content: string;
}

export function ChatScreen() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');

  const handleSend = async () => {
    if (!input.trim()) return;
    const userMsg: Message = { role: 'user', content: input };
    setMessages((prev) => [...prev, userMsg]);
    setInput('');

    try {
      const res = await fetch('/api/chat', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ message: input }),
      });
      const data = await res.json();
      setMessages((prev) => [
        ...prev,
        { role: 'assistant', content: data.reply || '(no response)' },
      ]);
    } catch {
      setMessages((prev) => [
        ...prev,
        { role: 'assistant', content: '(engine not connected)' },
      ]);
    }
  };

  return (
    <div className="flex h-full flex-col">
      <div className="border-b border-neutral-800 px-6 py-3">
        <h1 className="text-lg font-semibold text-neutral-200">Chat</h1>
      </div>
      <div className="flex-1 overflow-auto p-6">
        {messages.length === 0 ? (
          <div className="flex h-full items-center justify-center text-neutral-600">
            Start a conversation...
          </div>
        ) : (
          <div className="space-y-4">
            {messages.map((msg, i) => (
              <div
                key={i}
                className={msg.role === 'user' ? 'text-right' : 'text-left'}
              >
                <span
                  className={
                    msg.role === 'user'
                      ? 'inline-block rounded-lg bg-blue-600 px-3 py-2 text-white'
                      : 'inline-block rounded-lg bg-neutral-800 px-3 py-2 text-neutral-200'
                  }
                >
                  {msg.content}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
      <div className="border-t border-neutral-800 p-4">
        <div className="flex gap-2">
          <input
            type="text"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && handleSend()}
            placeholder="Type a message..."
            className="flex-1 rounded-lg border border-neutral-700 bg-neutral-900 px-4 py-2 text-sm text-neutral-200 outline-none focus:border-blue-500"
          />
          <button
            onClick={handleSend}
            className="rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white hover:bg-blue-700"
          >
            Send
          </button>
        </div>
      </div>
    </div>
  );
}
