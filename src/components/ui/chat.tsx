import { For, createSignal, onMount } from 'solid-js';
import { Send, ArrowLeft } from 'lucide-solid';
import { CommunityMember } from '../../lib/types';

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
    },
    {
        id: '1b',
        text: `If I die I will come back as a parrot... understand its me if i say "kawkaw" like this`,
        sender: 'them',
        timestamp: '10:01 AM'
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
      <div class="absolute inset-0 z-0 pointer-events-none overflow-hidden opacity-30">
        <div class="absolute top-[-10%] left-[-10%] w-[80%] h-[80%] bg-violet-600/20 rounded-full blur-[140px] animate-pulse" />
        <div class="absolute bottom-[-10%] right-[-10%] w-[80%] h-[80%] bg-amber-500/10 rounded-full blur-[140px] animate-pulse" style="animation-delay: 1s" />
        <div class="absolute top-[30%] right-[-5%] w-[50%] h-[50%] bg-blue-600/15 rounded-full blur-[120px] animate-pulse" style="animation-delay: 2s" />
      </div>

      {/* SVG Liquid Filters for UI Elements */}
      <svg class="absolute invisible w-0 h-0">
        <filter id="liquid-filter-chat" color-interpolation-filters="sRGB">
          <feGaussianBlur in="SourceGraphic" stdDeviation="4" result="blur" />
          <feColorMatrix in="blur" type="matrix" values="1 0 0 0 0  0 1 0 0 0  0 0 1 0 0  0 0 0 18 -8" result="goo" />
          <feComposite in="SourceGraphic" in2="goo" operator="atop" />
        </filter>
      </svg>

      {/* Floating Liquid Glass Header System */}
      <div class="absolute top-0 inset-x-0 z-50 flex items-center justify-between px-6 pt-14 pointer-events-none">
        {/* Back Button Glass Circle */}
        <button 
          onClick={props.onClose}
          class="w-14 h-14 rounded-full bg-zinc-900/40 backdrop-blur-2xl border border-white/10 flex items-center justify-center text-white/80 hover:text-white transition-all active:scale-90 shadow-2xl pointer-events-auto relative overflow-hidden group"
        >
          <div class="absolute inset-0 bg-white/5 opacity-0 group-hover:opacity-100 transition-opacity" />
          <ArrowLeft class="w-7 h-7" />
        </button>
        
        {/* Central Liquid Glass Tile */}
        <div class="flex-1 flex justify-center px-4 pointer-events-auto">
          <div class="relative w-full max-w-[220px] h-12">
            {/* The Glass Tile itself */}
            <div 
              class="absolute inset-0 bg-stone-950/40 backdrop-blur-3xl border border-white/10 rounded-full shadow-[0_12px_48px_rgba(0,0,0,0.6)] overflow-hidden"
            >
                {/* Subtle top gloss highlight */}
                <div class="absolute inset-0 bg-gradient-to-b from-white/20 to-transparent pointer-events-none opacity-40" />
            </div>

            {/* Tile Content */}
            <div class="relative h-full flex flex-col items-center justify-center text-center px-4">
                <span class="text-sm font-bold text-white tracking-tight leading-tight truncate w-full">{props.member.name}</span>
                <span class="text-[10px] text-zinc-400 font-medium opacity-80 leading-tight">last seen recently</span>
            </div>
          </div>
        </div>

        {/* Profile/More Glass Circle */}
        <div class="w-14 h-14 rounded-full p-[2px] bg-gradient-to-tr from-white/20 via-white/5 to-transparent shadow-2xl pointer-events-auto relative overflow-hidden">
            <div class="w-full h-full rounded-full overflow-hidden bg-zinc-900/40 backdrop-blur-xl border border-white/5">
                <img src={props.member.image_url} class="w-full h-full object-cover transition-transform duration-700 hover:scale-125" alt="" />
            </div>
        </div>
      </div>

      {/* Messages List - Full Screen Scroll Behind Header */}
      <div 
        ref={messagesContainer}
        class="flex-1 overflow-y-auto custom-scrollbar relative z-10"
      >
        {/* Top Scroll Blur Mask - Absolute so it doesn't push content */}
        <div class="absolute top-0 inset-x-0 h-48 z-20 pointer-events-none bg-gradient-to-b from-zinc-950 via-zinc-950/40 to-transparent backdrop-blur-xl [mask-image:linear-gradient(to_bottom,black_20%,transparent)]" />
        
        <div class="px-6 pt-32 pb-8 space-y-6">
            <div class="flex justify-center mb-4">
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
      </div>

      {/* Floating Input Area */}
      <div class="p-6 pb-12 bg-transparent z-50 relative pointer-events-none">
        <div class="max-w-2xl mx-auto flex items-center gap-3 bg-zinc-900/60 backdrop-blur-2xl border border-white/10 p-2 pl-6 rounded-[2.5rem] shadow-2xl focus-within:ring-2 ring-violet-500/20 transition-all group pointer-events-auto">
          <input 
            type="text" 
            placeholder="Write a message..."
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
