import { For, createSignal, onMount, Show } from 'solid-js';
import { Send, ArrowLeft, ShieldCheck, MoreVertical } from 'lucide-solid';
import { CommunityMember } from './community-page';

interface Message {
  id: string;
  text: string;
  sender: 'me' | 'them';
  timestamp: string;
}

interface ChatInterfaceProps {
  member: CommunityMember;
  onClose: () => void;
  onSendMessage: (text: string) => void;
}

export const ChatInterface = (props: ChatInterfaceProps) => {
  const [messages, setMessages] = createSignal<Message[]>([
    {
      id: '1',
      text: `Hello, how can we help you today at ${props.member.name}? Our team is ready to coordinate any rescue efforts needed.`,
      sender: 'them',
      timestamp: '10:00 AM'
    }
  ]);
  const [inputText, setInputText] = createSignal('');
  let messagesContainer: HTMLDivElement | undefined;

  const handleSend = () => {
    if (!inputText().trim()) return;
    
    const newMessage: Message = {
      id: Date.now().toString(),
      text: inputText(),
      sender: 'me',
      timestamp: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
    };
    
    setMessages([...messages(), newMessage]);
    props.onSendMessage(inputText());
    setInputText('');

    // Scroll to bottom
    setTimeout(() => {
      if (messagesContainer) {
        messagesContainer.scrollTop = messagesContainer.scrollHeight;
      }
    }, 50);

    // Mock reply
    setTimeout(() => {
        const reply: Message = {
            id: (Date.now() + 1).toString(),
            text: "Coordinates received. Our nearest responder has been notified. Please maintain visual contact with the animal.",
            sender: 'them',
            timestamp: new Date().toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
        };
        setMessages([...messages(), reply]);
        if (messagesContainer) {
            messagesContainer.scrollTop = messagesContainer.scrollHeight;
        }
    }, 1500);
  };

  onMount(() => {
    if (messagesContainer) {
      messagesContainer.scrollTop = messagesContainer.scrollHeight;
    }
  });

  return (
    <div class="fixed inset-0 z-[100] bg-zinc-950 flex flex-col animate-in slide-in-from-right duration-500 font-sans overflow-hidden">
      {/* Telegram-style Liquid Animated Background */}
      <div class="absolute inset-0 z-0 pointer-events-none overflow-hidden opacity-40">
        <div class="absolute top-[-10%] left-[-10%] w-[60%] h-[60%] bg-violet-600/30 rounded-full blur-[120px] animate-pulse" />
        <div class="absolute bottom-[-10%] right-[-10%] w-[60%] h-[60%] bg-amber-500/20 rounded-full blur-[120px] animate-pulse" style="animation-delay: 1s" />
        <div class="absolute top-[30%] right-[-5%] w-[40%] h-[40%] bg-blue-600/20 rounded-full blur-[100px] animate-pulse" style="animation-delay: 2s" />
      </div>

      {/* SVG Liquid Filters for UI Elements */}
      <svg class="absolute invisible w-0 h-0">
        <filter id="liquid-filter-chat" color-interpolation-filters="sRGB">
          <feGaussianBlur in="SourceGraphic" stdDeviation="4" result="blur" />
          <feColorMatrix in="blur" type="matrix" values="1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 18 -8" result="goo" />
          <feComposite in="SourceGraphic" in2="goo" operator="atop" />
        </filter>
      </svg>

      {/* Luxury Liquid Glass Header */}
      <div class="px-6 pt-14 pb-4 backdrop-blur-3xl bg-zinc-950/40 border-b border-white/5 flex items-center gap-4 z-50 relative">
        <button 
          onClick={props.onClose}
          class="w-10 h-10 rounded-full bg-white/5 border border-white/10 flex items-center justify-center text-white/70 hover:text-white transition-all active:scale-90"
        >
          <ArrowLeft class="w-5 h-5" />
        </button>
        
        <div class="flex-1 flex items-center gap-3">
          <div class="relative">
            <div class="w-11 h-11 rounded-full overflow-hidden border border-white/20 bg-zinc-900 shadow-xl">
              <img src={props.member.image_url} class="w-full h-full object-cover" alt="" />
            </div>
            <div class="absolute -bottom-0.5 -right-0.5 w-3.5 h-3.5 bg-emerald-500 rounded-full border-2 border-zinc-950 animate-pulse" />
          </div>
          <div class="flex flex-col">
            <div class="flex items-center gap-1.5">
              <span class="font-bold text-base text-white tracking-tight">{props.member.name}</span>
              <Show when={props.member.member_type === 'Vet'}>
                <div class="w-3.5 h-3.5 text-blue-400">
                  <svg viewBox="0 0 24 24" fill="currentColor"><path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41L9 16.17z"/></svg>
                </div>
              </Show>
            </div>
            <span class="text-[10px] font-bold text-zinc-500 uppercase tracking-widest leading-none">Online Coordination</span>
          </div>
        </div>

        <button class="w-10 h-10 rounded-full bg-white/5 border border-white/10 flex items-center justify-center text-white/40 hover:text-white transition-all active:scale-95">
          <MoreVertical class="w-5 h-5" />
        </button>
      </div>

      {/* Trust Sub-Header */}
      <div class="px-6 py-2 bg-violet-600/10 backdrop-blur-md flex items-center justify-center gap-2 border-b border-white/5 z-40 relative">
        <ShieldCheck class="w-3.5 h-3.5 text-violet-400" />
        <span class="text-[10px] font-bold text-violet-300/80 uppercase tracking-[0.15em]">Official Rescue Coordination Channel</span>
      </div>

      {/* Messages List - Glass Bubbles */}
      <div 
        ref={messagesContainer}
        class="flex-1 overflow-y-auto px-6 py-8 space-y-6 custom-scrollbar relative z-10"
      >
        <div class="flex justify-center mb-8">
            <span class="px-3 py-1 bg-white/5 backdrop-blur-xl border border-white/5 rounded-full text-[10px] font-bold text-zinc-500 uppercase tracking-widest">Today</span>
        </div>

        <For each={messages()}>
          {(msg) => (
            <div class={`flex ${msg.sender === 'me' ? 'justify-end' : 'justify-start'} animate-in fade-in slide-in-from-bottom-2 duration-300`}>
              <div class={`max-w-[85%] space-y-1`}>
                <div class={`px-4 py-3 rounded-[1.25rem] text-[15px] font-medium shadow-2xl relative overflow-hidden group transition-all hover:scale-[1.02] ${
                  msg.sender === 'me' 
                    ? 'bg-violet-600/40 backdrop-blur-xl border border-white/10 text-white rounded-tr-none' 
                    : 'bg-zinc-900/40 backdrop-blur-xl border border-white/5 text-zinc-200 rounded-tl-none'
                }`}>
                  {/* Subtle highlight glare on bubbles */}
                  <div class="absolute inset-0 bg-gradient-to-tr from-white/0 via-white/5 to-white/0 opacity-50 pointer-events-none" />
                  
                  {msg.text}

                  {/* Message Time - Subtle bottom right */}
                  <div class={`text-[9px] font-bold mt-1.5 opacity-40 uppercase tracking-tighter ${
                    msg.sender === 'me' ? 'text-right' : 'text-left'
                  }`}>
                    {msg.timestamp}
                  </div>
                </div>
              </div>
            </div>
          )}
        </For>
      </div>

      {/* Input Area - Floating Glass Pill */}
      <div class="p-6 pb-10 bg-transparent z-50 relative">
        <div class="max-w-2xl mx-auto flex items-center gap-3 bg-zinc-900/60 backdrop-blur-2xl border border-white/10 p-2 pl-6 rounded-[2.5rem] shadow-2xl focus-within:ring-2 ring-violet-500/20 transition-all group">
          <input 
            type="text" 
            placeholder="Type your message..."
            value={inputText()}
            onInput={(e) => setInputText(e.currentTarget.value)}
            onKeyPress={(e) => e.key === 'Enter' && handleSend()}
            class="flex-1 bg-transparent border-none outline-none text-[15px] text-white py-3 placeholder:text-zinc-600 font-medium"
          />
          <button 
            onClick={handleSend}
            disabled={!inputText().trim()}
            class={`w-12 h-12 rounded-full flex items-center justify-center transition-all active:scale-90 shadow-xl relative overflow-hidden ${
              inputText().trim() 
                ? 'bg-violet-600 text-white' 
                : 'bg-zinc-800 text-zinc-600 opacity-50'
            }`}
          >
            {/* Liquid effect for send button */}
            <div 
                class="absolute inset-0 bg-white/10 opacity-0 group-hover:opacity-100 transition-opacity"
                style="filter: url(#liquid-filter-chat)"
            />
            <Send class="w-5 h-5 relative z-10 fill-current ml-0.5" />
          </button>
        </div>
      </div>
    </div>
  );
};
