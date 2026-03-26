/// <reference types="vite/client" />

/**
 * Messaging Service for WarmStreet
 *
 * Handles:
 * - Sending encrypted messages via Appwrite
 * - Receiving messages via realtime subscriptions
 * - Managing conversations
 * - Message persistence
 *
 * Encryption: Libsodium (X25519 + XSalsa20-Poly1305)
 */

import { ID, Query, RealtimeResponseEvent } from 'appwrite';
import client, { account, databases } from './appwrite';
import {
  encryptMessage,
  decryptMessage,
  getUserPublicKey,
  initializeUserEncryption,
  getPrivateKey,
  initSodium,
} from './encryption';
import type { Message, Conversation, CommunityMember } from './types';

// Appwrite Database Configuration
const DATABASE_ID = 'warmstreet_messaging';
const CONVERSATIONS_COLLECTION_ID = 'conversations';
const MESSAGES_COLLECTION_ID = 'messages';

// Cache for current user
let currentUserId: string | null = null;
let currentUserKeys: { publicKey: string; hasPrivateKey: boolean } | null = null;

/**
 * Initialize messaging service for current user
 */
export async function initializeMessaging(): Promise<void> {
  try {
    await initSodium();

    const user = await account.get();
    currentUserId = user.$id;
    currentUserKeys = await initializeUserEncryption(user.$id);

    // Store public key in user's prefs for others to access
    try {
      const prefs: any = user.prefs || {};
      if (!prefs.encryption_public_key) {
        await account.updatePrefs({
          ...prefs,
          encryption_public_key: currentUserKeys.publicKey,
        });
      }
    } catch (err) {
      if (import.meta.env.DEV) console.warn('⚠️ Could not store public key in prefs:', err);
    }

    if (import.meta.env.DEV) console.log('✅ Messaging service initialized with libsodium');
  } catch (error) {
    if (import.meta.env.DEV) console.error('❌ Failed to initialize messaging:', error);
    throw error;
  }
}

/**
 * Clean up messaging service on logout
 */
export async function cleanupMessaging(): Promise<void> {
  if (currentUserId) {
    const { cleanupEncryptionData } = await import('./encryption');
    await cleanupEncryptionData(currentUserId);
  }
  currentUserId = null;
  currentUserKeys = null;
  if (import.meta.env.DEV) console.log('🧹 Messaging service cleaned up');
}

/**
 * Get a user's public key from their Appwrite prefs
 */
export async function getUserPublicKeyFromPrefs(userId: string): Promise<string | null> {
  try {
    // Try to get from localStorage first (cached)
    const cached = await getUserPublicKey(userId);
    if (cached) return cached;

    // For production, you'd use a Cloud Function to fetch user prefs
    // For now, try localStorage
    return localStorage.getItem(`warmstreet_public_key_${userId}`);
  } catch (error) {
    if (import.meta.env.DEV) console.error('❌ Failed to get public key:', error);
    return null;
  }
}

/**
 * Get or create a conversation with another user
 */
export async function getOrCreateConversation(
  otherUserId: string,
  otherUser: CommunityMember
): Promise<Conversation> {
  if (!currentUserId) {
    throw new Error('User not authenticated');
  }

  try {
    // Generate deterministic conversation ID
    const { getConversationId } = await import('./encryption');
    const convId = getConversationId(currentUserId, otherUserId);

    // Check if conversation already exists
    const conversations = await databases.listDocuments(
      DATABASE_ID,
      CONVERSATIONS_COLLECTION_ID,
      [
        Query.equal('participant_ids', [currentUserId, otherUserId]),
        Query.limit(1),
      ]
    );

    if (conversations.documents.length > 0) {
      const doc = conversations.documents[0];
      return {
        id: doc.$id,
        participant_ids: doc.participant_ids,
        participants: [otherUser],
        last_message: doc.last_message ? deserializeMessage(doc.last_message) : undefined,
        unread_count: doc.unread_count || 0,
        updated_at: doc.$updatedAt,
      };
    }

    // Create new conversation
    const user = await account.get();
    const newConversation = await databases.createDocument(
      DATABASE_ID,
      CONVERSATIONS_COLLECTION_ID,
      convId, // Use deterministic ID
      {
        participant_ids: [currentUserId, otherUserId],
        participant_details: [
          {
            user_id: currentUserId,
            name: user.name,
          },
          {
            user_id: otherUserId,
            name: otherUser.name,
            member_type: otherUser.member_type,
            avatar: otherUser.image_url,
          },
        ],
        last_message: null,
        unread_count: 0,
      }
    );

    return {
      id: newConversation.$id,
      participant_ids: newConversation.participant_ids,
      participants: [otherUser],
      unread_count: 0,
      updated_at: newConversation.$updatedAt,
    };
  } catch (error) {
    if (import.meta.env.DEV) console.error('❌ Failed to get/create conversation:', error);
    throw error;
  }
}

/**
 * Send an encrypted message
 */
export async function sendMessage(
  conversationId: string,
  recipientId: string,
  messageText: string
): Promise<Message> {
  if (!currentUserId || !currentUserKeys) {
    throw new Error('Messaging service not initialized');
  }

  try {
    // Get recipient's public key
    let publicKey = await getUserPublicKeyFromPrefs(recipientId);
    if (!publicKey) {
      throw new Error('Cannot get recipient public key - they may need to log in first');
    }

    // Get sender's private key
    const privateKey = await getPrivateKey(currentUserId);
    if (!privateKey) {
      throw new Error('Sender private key not found');
    }

    // Encrypt the message with libsodium
    const { encryptedContent, nonce } = await encryptMessage(
      messageText,
      privateKey,
      publicKey
    );

    // Create message document
    const user = await account.get();
    const messageDoc = await databases.createDocument(
      DATABASE_ID,
      MESSAGES_COLLECTION_ID,
      ID.unique(),
      {
        conversation_id: conversationId,
        sender_id: currentUserId,
        receiver_id: recipientId,
        encrypted_content: encryptedContent,
        nonce: nonce,
        read: false,
        sender_name: user.name,
      }
    );

    // Update conversation's last message
    await databases.updateDocument(
      DATABASE_ID,
      CONVERSATIONS_COLLECTION_ID,
      conversationId,
      {
        last_message: {
          id: messageDoc.$id,
          encrypted_content: encryptedContent,
          created_at: messageDoc.$createdAt,
          sender_id: currentUserId,
          sender_name: user.name,
        },
        updated_at: new Date().toISOString(),
      }
    );

    const message: Message = {
      id: messageDoc.$id,
      conversation_id: conversationId,
      sender_id: currentUserId,
      receiver_id: recipientId,
      encrypted_content: encryptedContent,
      signature: '', // Libsodium includes auth in ciphertext
      created_at: messageDoc.$createdAt,
      read: false,
      sender_name: messageDoc.sender_name,
    };

    if (import.meta.env.DEV) console.log('✅ Message sent and encrypted with libsodium');
    return message;
  } catch (error) {
    if (import.meta.env.DEV) console.error('❌ Failed to send message:', error);
    throw error;
  }
}

/**
 * Get messages for a conversation
 */
export async function getConversationMessages(
  conversationId: string,
  limit: number = 50
): Promise<Message[]> {
  if (!currentUserId || !currentUserKeys) {
    throw new Error('Messaging service not initialized');
  }

  try {
    const messages = await databases.listDocuments(
      DATABASE_ID,
      MESSAGES_COLLECTION_ID,
      [
        Query.equal('conversation_id', conversationId),
        Query.orderAsc('$createdAt'),
        Query.limit(limit),
      ]
    );

    // Decrypt all messages
    const decryptedMessages: Message[] = [];
    for (const doc of messages.documents) {
      try {
        const decryptedMessage = await decryptMessageDoc(doc);
        if (decryptedMessage) {
          decryptedMessages.push(decryptedMessage);
        }
      } catch (error) {
        if (import.meta.env.DEV) console.error('⚠️ Failed to decrypt message:', doc.$id, error);
        decryptedMessages.push({
          ...doc,
          decrypted_content: '[Unable to decrypt]',
        } as any);
      }
    }

    return decryptedMessages;
  } catch (error) {
    if (import.meta.env.DEV) console.error('❌ Failed to get messages:', error);
    throw error;
  }
}

/**
 * Subscribe to new messages in a conversation (realtime)
 */
export function subscribeToConversation(
  conversationId: string,
  onMessage: (message: Message) => void
): () => void {
  const unsubscribe = client.subscribe(
    `databases.${DATABASE_ID}.collections.${MESSAGES_COLLECTION_ID}.documents`,
    (response: RealtimeResponseEvent<any>) => {
      if (response.payload.conversation_id === conversationId) {
        if (response.events.includes('databases.*.collections.*.documents.*.create')) {
          decryptMessageDoc(response.payload).then((decrypted) => {
            if (decrypted) {
              onMessage(decrypted);
            }
          });
        } else if (response.events.includes('databases.*.collections.*.documents.*.update')) {
          // Warning: Bubbling update events causes duplicate appending in the UI.
          // We ignore them for now until ChatInterface supports replacing messages (e.g., for read receipts).
        }
      }
    }
  );

  return unsubscribe;
}

/**
 * Mark a message as read
 */
export async function markMessageAsRead(messageId: string): Promise<void> {
  try {
    await databases.updateDocument(
      DATABASE_ID,
      MESSAGES_COLLECTION_ID,
      messageId,
      {
        read: true,
      }
    );
  } catch (error) {
    if (import.meta.env.DEV) console.error('❌ Failed to mark message as read:', error);
  }
}

/**
 * Get all conversations for current user
 */
export async function getConversations(): Promise<Conversation[]> {
  if (!currentUserId) {
    throw new Error('User not authenticated');
  }

  try {
    const conversations = await databases.listDocuments(
      DATABASE_ID,
      CONVERSATIONS_COLLECTION_ID,
      [
        Query.equal('participant_ids', currentUserId),
        Query.orderDesc('updated_at'),
      ]
    );

    return conversations.documents.map((doc) => ({
      id: doc.$id,
      participant_ids: doc.participant_ids,
      participants: doc.participant_details || [],
      last_message: doc.last_message ? deserializeMessage(doc.last_message) : undefined,
      unread_count: doc.unread_count || 0,
      updated_at: doc.$updatedAt,
    }));
  } catch (error) {
    if (import.meta.env.DEV) console.error('❌ Failed to get conversations:', error);
    return [];
  }
}

/**
 * Helper: Decrypt a message document
 */
async function decryptMessageDoc(doc: any): Promise<Message | null> {
  if (!currentUserId || !currentUserKeys) {
    return null;
  }

  try {
    const privateKey = await getPrivateKey(currentUserId);
    if (!privateKey) {
      return null;
    }

    // Get sender's public key
    const senderPublicKey = await getUserPublicKeyFromPrefs(doc.sender_id);
    if (!senderPublicKey) {
      if (import.meta.env.DEV) console.warn('⚠️ Cannot get sender public key');
      return null;
    }

    if (!doc.nonce) {
      if (import.meta.env.DEV) console.warn('⚠️ Missing nonce in message. Cannot decrypt.');
      return null;
    }

    const decryptedContent = await decryptMessage(
      doc.encrypted_content,
      doc.nonce,
      privateKey,
      senderPublicKey
    );

    return {
      id: doc.$id,
      conversation_id: doc.conversation_id,
      sender_id: doc.sender_id,
      receiver_id: doc.receiver_id,
      encrypted_content: doc.encrypted_content,
      signature: '',
      decrypted_content: decryptedContent,
      created_at: doc.$createdAt,
      read: doc.read,
      sender_name: doc.sender_name,
    };
  } catch (error) {
    if (import.meta.env.DEV) console.error('❌ Failed to decrypt message:', error);
    return null;
  }
}

/**
 * Helper: Deserialize message from Appwrite document
 */
function deserializeMessage(data: any): Message {
  return {
    id: data.id,
    conversation_id: data.conversation_id,
    sender_id: data.sender_id,
    receiver_id: data.receiver_id,
    encrypted_content: data.encrypted_content,
    signature: '',
    created_at: data.created_at,
    read: data.read,
    sender_name: data.sender_name,
  };
}

/**
 * Check if messaging is initialized
 */
export function isMessagingInitialized(): boolean {
  return !!currentUserId && !!currentUserKeys;
}

/**
 * Get current user ID
 */
export function getCurrentUserId(): string | null {
  return currentUserId;
}
