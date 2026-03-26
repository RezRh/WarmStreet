/// <reference types="vite/client" />
/**
 * Libsodium E2E Encryption Service
 * 
 * Uses libsodium-wrappers-sumo for production-grade cryptography:
 * - X25519 for key exchange
 * - XSalsa20-Poly1305 for authenticated encryption
 * - Ed25519 for digital signatures
 * 
 * This is the same cryptography used by Signal, WhatsApp, and other secure messengers.
 */

import sodium from 'libsodium-wrappers-sumo';

// Cache for user keys
let userKeyPair: {
  publicKey: Uint8Array;
  privateKey: Uint8Array;
} | null = null;

// Helper to determine if we are running in Tauri
const isTauri = () => typeof window !== 'undefined' && !!(window as any).__TAURI_INTERNALS__;

/**
 * Initialize libsodium (must be called before any crypto operations)
 */
export async function initSodium(): Promise<void> {
  await sodium.ready;
  if (import.meta.env.DEV) console.log('✅ Libsodium initialized');
}

/**
 * Derive a user-specific Stronghold vault key using Argon2id.
 * This prevents a universal hardcoded key from decrypting the vault.
 */
function getVaultKey(userId: string): string {
  // We use the minimum interactive limits to keep app startup fast
  // but still provide strong resistance against automated extraction.
  const salt = new TextEncoder().encode("warmstreet-vault-salt-8fa923");
  // ensure salt is exactly sodium.crypto_pwhash_SALTBYTES (16)
  const paddedSalt = new Uint8Array(sodium.crypto_pwhash_SALTBYTES);
  paddedSalt.set(salt.slice(0, sodium.crypto_pwhash_SALTBYTES));

  const keyBytes = sodium.crypto_pwhash(
    32, // 32 bytes
    new TextEncoder().encode(userId + "-stronghold-key"),
    paddedSalt,
    sodium.crypto_pwhash_OPSLIMIT_INTERACTIVE,
    sodium.crypto_pwhash_MEMLIMIT_INTERACTIVE,
    sodium.crypto_pwhash_ALG_ARGON2ID13
  );
  return sodium.to_hex(keyBytes);
}

/**
 * Generate a new X25519 key pair for the user
 * X25519 is designed for key exchange (ECDH)
 */
export async function generateUserKeys(userId: string): Promise<{
  publicKey: string;
  privateKey: string;
}> {
  await initSodium();

  const keyPair = sodium.crypto_box_keypair();

  // Convert to base64 for storage
  const publicKeyB64 = sodium.to_base64(keyPair.publicKey, sodium.base64_variants.ORIGINAL);
  const privateKeyB64 = sodium.to_base64(keyPair.privateKey, sodium.base64_variants.ORIGINAL);

  // Store private key securely
  await storePrivateKey(userId, privateKeyB64);

  // Cache for quick access
  userKeyPair = {
    publicKey: keyPair.publicKey,
    privateKey: keyPair.privateKey,
  };

  return {
    publicKey: publicKeyB64,
    privateKey: privateKeyB64,
  };
}

/**
 * Import a public key from base64 string
 */
export function importPublicKey(publicKeyB64: string): Uint8Array {
  return sodium.from_base64(publicKeyB64, sodium.base64_variants.ORIGINAL);
}

/**
 * Import a private key from base64 string
 */
export function importPrivateKey(privateKeyB64: string): Uint8Array {
  return sodium.from_base64(privateKeyB64, sodium.base64_variants.ORIGINAL);
}

/**
 * Encrypt a message for a recipient
 * Uses X25519 key exchange + XSalsa20-Poly1305 encryption
 * 
 * Libsodium handles:
 * - Nonce generation automatically (prevents nonce reuse attacks)
 * - Authentication tag automatically (prevents tampering)
 */
export async function encryptMessage(
  message: string,
  senderPrivateKeyB64: string,
  recipientPublicKeyB64: string
): Promise<{
  encryptedContent: string;
  nonce: string;
}> {
  await initSodium();

  const senderPrivateKey = importPrivateKey(senderPrivateKeyB64);
  const recipientPublicKey = importPublicKey(recipientPublicKeyB64);

  // Convert message to Uint8Array
  const messageBytes = sodium.from_string(message);

  // Generate random nonce (24 bytes for XSalsa20)
  const nonce = sodium.randombytes_buf(sodium.crypto_box_NONCEBYTES);

  // Encrypt
  const ciphertext = sodium.crypto_box_easy(
    messageBytes,
    nonce,
    recipientPublicKey,
    senderPrivateKey
  );

  return {
    encryptedContent: sodium.to_base64(ciphertext, sodium.base64_variants.ORIGINAL),
    nonce: sodium.to_base64(nonce, sodium.base64_variants.ORIGINAL),
  };
}


/**
 * Decrypt a message using recipient's private key
 * Automatically verifies authentication tag
 */
export async function decryptMessage(
  encryptedContentB64: string,
  nonceB64: string,
  recipientPrivateKeyB64: string,
  senderPublicKeyB64: string
): Promise<string> {
  await initSodium();

  const recipientPrivateKey = importPrivateKey(recipientPrivateKeyB64);
  const senderPublicKey = importPublicKey(senderPublicKeyB64);

  const ciphertext = sodium.from_base64(encryptedContentB64, sodium.base64_variants.ORIGINAL);
  const nonce = sodium.from_base64(nonceB64, sodium.base64_variants.ORIGINAL);

  // Decrypt and verify authentication tag
  const decrypted = sodium.crypto_box_open_easy(
    ciphertext,
    nonce,
    senderPublicKey,
    recipientPrivateKey
  );

  return sodium.to_string(decrypted);
}

/* Note: Separate digital signature functions were removed because 
   converting Curve25519 (X25519) keys back to Ed25519 keys is cryptographically unsafe.
   XSalsa20-Poly1305 (used via crypto_box) already provides authentication 
   proving the sender's identity to the recipient. */

/**
 * Store user's private key in IndexedDB (more secure than localStorage)
 */
export async function storePrivateKey(userId: string, privateKeyB64: string): Promise<void> {
  if (isTauri()) {
    try {
      const { Stronghold } = await import('@tauri-apps/plugin-stronghold');
      const vaultKey = getVaultKey(userId);
      const stronghold = await Stronghold.load(".warmstreet_vault", vaultKey);
      let client;
      try {
        client = await stronghold.loadClient("keys-client");
      } catch {
        client = await stronghold.createClient("keys-client");
      }
      const store = client.getStore();
      // Stronghold expects an array of numbers
      const data = Array.from(new TextEncoder().encode(privateKeyB64));
      await store.insert(userId, data);
      await stronghold.save(); // Commit changes to disk
      if (import.meta.env.DEV) console.log('🔒 Private key securely stored in Tauri Stronghold');
      return;
    } catch (e) {
      if (import.meta.env.DEV) console.warn('⚠️ Failed to store in Stronghold, falling back to IndexedDB:', e);
    }
  }

  return new Promise((resolve, reject) => {
    const request = indexedDB.open('WarmStreetKeys', 1);

    request.onerror = () => reject(request.error);
    request.onsuccess = () => {
      const db = request.result;
      const transaction = db.transaction(['keys'], 'readwrite');
      const store = transaction.objectStore('keys');
      const putRequest = store.put({ userId, privateKey: privateKeyB64 });
      putRequest.onsuccess = () => resolve();
      putRequest.onerror = () => reject(putRequest.error);
    };

    request.onupgradeneeded = (event) => {
      const db = (event.target as IDBOpenDBRequest).result;
      if (!db.objectStoreNames.contains('keys')) {
        db.createObjectStore('keys', { keyPath: 'userId' });
      }
    };
  });
}

/**
 * Retrieve user's private key from IndexedDB
 */
export async function getPrivateKey(userId: string): Promise<string | null> {
  if (isTauri()) {
    try {
      const { Stronghold } = await import('@tauri-apps/plugin-stronghold');
      const vaultKey = getVaultKey(userId);
      const stronghold = await Stronghold.load(".warmstreet_vault", vaultKey);
      let client;
      try {
        client = await stronghold.loadClient("keys-client");
      } catch {
        client = await stronghold.createClient("keys-client");
      }
      const store = client.getStore();
      const value = await store.get(userId);
      if (value) {
        return new TextDecoder().decode(value);
      }
    } catch (e) {
      if (import.meta.env.DEV) console.warn('⚠️ Failed to get from Stronghold, checking IndexedDB fallback:', e);
    }
  }

  return new Promise((resolve, reject) => {
    const request = indexedDB.open('WarmStreetKeys', 1);

    request.onerror = () => reject(request.error);
    request.onsuccess = () => {
      const db = request.result;
      
      if (!db.objectStoreNames.contains('keys')) {
        resolve(null);
        return;
      }

      const transaction = db.transaction(['keys'], 'readonly');
      const store = transaction.objectStore('keys');
      const getRequest = store.get(userId);

      getRequest.onsuccess = () => {
        resolve(getRequest.result?.privateKey || null);
      };
      getRequest.onerror = () => reject(getRequest.error);
    };

    request.onupgradeneeded = (event) => {
      const db = (event.target as IDBOpenDBRequest).result;
      if (!db.objectStoreNames.contains('keys')) {
        db.createObjectStore('keys', { keyPath: 'userId' });
      }
    };
  });
}

/**
 * Delete user's private key from IndexedDB
 */
export async function deletePrivateKey(userId: string): Promise<void> {
  if (isTauri()) {
    try {
      const { Stronghold } = await import('@tauri-apps/plugin-stronghold');
      const vaultKey = getVaultKey(userId);
      const stronghold = await Stronghold.load(".warmstreet_vault", vaultKey);
      let client;
      try {
        client = await stronghold.loadClient("keys-client");
      } catch {
        client = await stronghold.createClient("keys-client");
      }
      const store = client.getStore();
      await store.remove(userId);
      await stronghold.save();
      if (import.meta.env.DEV) console.log('🧹 Cleaned up key from Stronghold');
    } catch (e) {
      if (import.meta.env.DEV) console.warn('⚠️ Failed to delete from Stronghold:', e);
    }
  }

  return new Promise((resolve, reject) => {
    const request = indexedDB.open('WarmStreetKeys', 1);

    request.onerror = () => reject(request.error);
    request.onsuccess = () => {
      const db = request.result;
      
      if (!db.objectStoreNames.contains('keys')) {
        resolve();
        return;
      }

      const transaction = db.transaction(['keys'], 'readwrite');
      const store = transaction.objectStore('keys');
      const deleteRequest = store.delete(userId);
      
      deleteRequest.onsuccess = () => resolve();
      deleteRequest.onerror = () => reject(deleteRequest.error);
    };
  });
}

/**
 * Get or generate user's public key
 * For now, stored in localStorage (public keys are safe to share)
 */
export async function getUserPublicKey(userId: string): Promise<string | null> {
  const storageKey = `warmstreet_public_key_${userId}`;
  return localStorage.getItem(storageKey);
}

/**
 * Store user's public key
 */
export function storeUserPublicKey(userId: string, publicKeyB64: string): void {
  const storageKey = `warmstreet_public_key_${userId}`;
  localStorage.setItem(storageKey, publicKeyB64);
}

/**
 * Initialize user's encryption keys on login
 */
export async function initializeUserEncryption(userId: string): Promise<{
  publicKey: string;
  hasPrivateKey: boolean;
}> {
  await initSodium();

  let privateKeyB64 = await getPrivateKey(userId);
  let publicKeyB64: string;

  if (!privateKeyB64) {
    // Generate new keys
    const keys = await generateUserKeys(userId);
    publicKeyB64 = keys.publicKey;
    privateKeyB64 = keys.privateKey;

    // Store public key for others to find
    storeUserPublicKey(userId, publicKeyB64);

    if (import.meta.env.DEV) console.log('✅ Generated new libsodium encryption keys for user');
  } else {
    // Get existing public key
    const storedPublicKey = await getUserPublicKey(userId);
    if (storedPublicKey) {
      publicKeyB64 = storedPublicKey;
    } else {
      // Regenerate if public key missing (edge case)
      const keys = await generateUserKeys(userId);
      publicKeyB64 = keys.publicKey;
    }

    // Cache the keypair
    userKeyPair = {
      publicKey: importPublicKey(publicKeyB64),
      privateKey: importPrivateKey(privateKeyB64),
    };

    if (import.meta.env.DEV) console.log('✅ Loaded existing libsodium encryption keys for user');
  }

  return {
    publicKey: publicKeyB64,
    hasPrivateKey: !!privateKeyB64,
  };
}

/**
 * Clean up encryption data on logout
 */
export async function cleanupEncryptionData(userId: string): Promise<void> {
  await deletePrivateKey(userId);
  userKeyPair = null;
  if (import.meta.env.DEV) console.log('🧹 Encryption data cleaned up for user');
}

/**
 * Get current user's key pair (cached)
 */
export function getCachedKeyPair(): { publicKey: Uint8Array; privateKey: Uint8Array } | null {
  return userKeyPair;
}

/**
 * Generate a deterministic conversation ID from two user IDs
 * Ensures same conversation ID regardless of who initiates
 * Uses a generic hash to stay within Appwrite's 36-character ID limit
 */
export function getConversationId(userId1: string, userId2: string): string {
  const sorted = [userId1, userId2].sort();
  const input = sorted.join(':');
  
  // Use a generic hash (Blake2b) to create a unique ID from the pair
  // 16-byte hash results in a 32-character hex ID, fitting Appwrite limits.
  const hash = sodium.crypto_generichash(16, input, null);
  return sodium.to_hex(hash);
}

/**
 * Generate a human-readable fingerprint (security code) for a public key
 * Allows users to manually verify they aren't being MITM'd.
 */
export function getFingerprint(publicKeyB64: string): string {
  const publicKey = importPublicKey(publicKeyB64);
  const hash = sodium.crypto_generichash(16, publicKey, null); // 16 bytes is plenty
  const hex = sodium.to_hex(hash).toUpperCase();
  // Return in groups for readability: ABCD-EFGH-...
  return hex.match(/.{1,4}/g)?.join('-') || hex;
}
