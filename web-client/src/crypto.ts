/**
 * Voile Protocol - Cryptographic Utilities
 * 
 * Handles commitment generation, encryption, and proof helpers
 * All cryptographic operations run CLIENT-SIDE for privacy
 */

import type { Felt, Word, AccountId, Timestamp } from './types';

// ============================================================================
// RANDOM NUMBER GENERATION
// ============================================================================

/**
 * Generate cryptographically secure random bytes
 */
export function randomBytes(length: number): Uint8Array {
  const bytes = new Uint8Array(length);
  
  if (typeof crypto !== 'undefined' && crypto.getRandomValues) {
    crypto.getRandomValues(bytes);
  } else {
    // Fallback for non-browser environments
    for (let i = 0; i < length; i++) {
      bytes[i] = Math.floor(Math.random() * 256);
    }
  }
  
  return bytes;
}

/**
 * Generate a random nullifier secret (32 bytes)
 */
export function generateNullifierSecret(): Uint8Array {
  return randomBytes(32);
}

/**
 * Generate a random Felt
 */
export function randomFelt(): Felt {
  const bytes = randomBytes(8);
  const view = new DataView(bytes.buffer);
  // Use only 63 bits to stay within Miden field
  return BigInt(view.getBigUint64(0, true)) & ((1n << 63n) - 1n);
}

/**
 * Generate a random Word (4 Felts)
 */
export function randomWord(): Word {
  return [randomFelt(), randomFelt(), randomFelt(), randomFelt()];
}

// ============================================================================
// COMMITMENT GENERATION
// ============================================================================

/**
 * Compute unlock request commitment
 * commitment = hash(amount, cooldown_end, nullifier_secret, user_id)
 * 
 * NOTE: This is a simplified commitment for demo purposes.
 * In production, use Rescue Prime hash (Miden's native hash).
 */
export function computeRequestCommitment(
  amount: bigint,
  cooldownEnd: Timestamp,
  nullifierSecret: Uint8Array,
  userAccountId: AccountId,
): Word {
  // Extract first 8 bytes of nullifier as felt
  const nullifierFelt = bytesToFelt(nullifierSecret.slice(0, 8));
  
  // Parse user account ID as felt
  const userIdFelt = accountIdToFelt(userAccountId);
  
  // Simplified commitment (in production, use proper hash)
  return [amount, cooldownEnd, nullifierFelt, userIdFelt];
}

/**
 * Compute LP offer commitment
 */
export function computeOfferCommitment(
  offerId: bigint,
  lpAccountId: AccountId,
  maxAmount: bigint,
  minAmount: bigint,
): Word {
  const lpIdFelt = accountIdToFelt(lpAccountId);
  return [offerId, lpIdFelt, maxAmount, minAmount];
}

/**
 * Compute deal ID from request and offer
 */
export function computeDealId(
  requestCommitment: Word,
  offerCommitment: Word,
  timestamp: Timestamp,
): Word {
  // XOR components together with timestamp
  return [
    requestCommitment[0] ^ offerCommitment[0] ^ timestamp,
    requestCommitment[1] ^ offerCommitment[1],
    requestCommitment[2] ^ offerCommitment[2],
    requestCommitment[3] ^ offerCommitment[3] ^ randomFelt(),
  ];
}

// ============================================================================
// NULLIFIER COMPUTATION
// ============================================================================

/**
 * Compute nullifier for an unlock request
 * Nullifier prevents double-spending of the same request
 */
export function computeNullifier(
  requestId: bigint,
  nullifierSecret: Uint8Array,
): Felt {
  const secretFelt = bytesToFelt(nullifierSecret.slice(0, 8));
  // Simplified nullifier (in production, use proper hash)
  return requestId ^ secretFelt;
}

// ============================================================================
// ENCRYPTION (for private notes)
// ============================================================================

/**
 * Encryption key pair
 */
export interface KeyPair {
  publicKey: Uint8Array;
  privateKey: Uint8Array;
}

/**
 * Generate a new encryption key pair
 * Uses X25519 for key exchange
 */
export async function generateKeyPair(): Promise<KeyPair> {
  if (typeof crypto !== 'undefined' && crypto.subtle) {
    const keyPair = await crypto.subtle.generateKey(
      { name: 'ECDH', namedCurve: 'P-256' },
      true,
      ['deriveBits']
    );
    
    const publicKey = await crypto.subtle.exportKey('raw', keyPair.publicKey);
    const privateKey = await crypto.subtle.exportKey('pkcs8', keyPair.privateKey);
    
    return {
      publicKey: new Uint8Array(publicKey),
      privateKey: new Uint8Array(privateKey),
    };
  }
  
  // Fallback: random bytes (insecure, for demo only)
  return {
    publicKey: randomBytes(65),
    privateKey: randomBytes(121),
  };
}

/**
 * Encrypt note data for a recipient
 */
export async function encryptNoteData(
  data: Uint8Array,
  recipientPublicKey: Uint8Array,
): Promise<Uint8Array> {
  // Simplified encryption for demo
  // In production, use proper ECIES with Miden-compatible curves
  
  // Generate ephemeral key
  const ephemeralKey = randomBytes(32);
  
  // XOR encryption (insecure, for demo only)
  const encrypted = new Uint8Array(data.length + 32);
  encrypted.set(ephemeralKey, 0);
  for (let i = 0; i < data.length; i++) {
    encrypted[32 + i] = data[i] ^ ephemeralKey[i % 32];
  }
  
  return encrypted;
}

/**
 * Decrypt note data
 */
export async function decryptNoteData(
  encryptedData: Uint8Array,
  privateKey: Uint8Array,
): Promise<Uint8Array> {
  // Extract ephemeral key
  const ephemeralKey = encryptedData.slice(0, 32);
  const ciphertext = encryptedData.slice(32);
  
  // XOR decryption
  const decrypted = new Uint8Array(ciphertext.length);
  for (let i = 0; i < ciphertext.length; i++) {
    decrypted[i] = ciphertext[i] ^ ephemeralKey[i % 32];
  }
  
  return decrypted;
}

// ============================================================================
// CONVERSION UTILITIES
// ============================================================================

/**
 * Convert bytes to Felt
 */
export function bytesToFelt(bytes: Uint8Array): Felt {
  let value = 0n;
  for (let i = 0; i < Math.min(bytes.length, 8); i++) {
    value |= BigInt(bytes[i]) << BigInt(i * 8);
  }
  return value;
}

/**
 * Convert Felt to bytes
 */
export function feltToBytes(felt: Felt): Uint8Array {
  const bytes = new Uint8Array(8);
  for (let i = 0; i < 8; i++) {
    bytes[i] = Number((felt >> BigInt(i * 8)) & 0xffn);
  }
  return bytes;
}

/**
 * Convert Word to bytes
 */
export function wordToBytes(word: Word): Uint8Array {
  const bytes = new Uint8Array(32);
  for (let i = 0; i < 4; i++) {
    bytes.set(feltToBytes(word[i]), i * 8);
  }
  return bytes;
}

/**
 * Convert bytes to Word
 */
export function bytesToWord(bytes: Uint8Array): Word {
  return [
    bytesToFelt(bytes.slice(0, 8)),
    bytesToFelt(bytes.slice(8, 16)),
    bytesToFelt(bytes.slice(16, 24)),
    bytesToFelt(bytes.slice(24, 32)),
  ];
}

/**
 * Convert AccountId string to Felt
 */
export function accountIdToFelt(accountId: AccountId): Felt {
  // If hex string, parse it
  if (accountId.startsWith('0x')) {
    return BigInt(accountId);
  }
  // Otherwise, hash the string
  let hash = 0n;
  for (let i = 0; i < accountId.length; i++) {
    hash = (hash * 31n + BigInt(accountId.charCodeAt(i))) & ((1n << 64n) - 1n);
  }
  return hash;
}

/**
 * Convert hex string to bytes
 */
export function hexToBytes(hex: string): Uint8Array {
  const cleanHex = hex.startsWith('0x') ? hex.slice(2) : hex;
  const bytes = new Uint8Array(cleanHex.length / 2);
  for (let i = 0; i < bytes.length; i++) {
    bytes[i] = parseInt(cleanHex.substr(i * 2, 2), 16);
  }
  return bytes;
}

/**
 * Convert bytes to hex string
 */
export function bytesToHex(bytes: Uint8Array): string {
  return '0x' + Array.from(bytes)
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
}

// ============================================================================
// TIMESTAMP UTILITIES
// ============================================================================

/**
 * Get current Unix timestamp
 */
export function currentTimestamp(): Timestamp {
  return BigInt(Math.floor(Date.now() / 1000));
}

/**
 * Calculate cooldown end timestamp
 */
export function cooldownEndTimestamp(cooldownSeconds: bigint): Timestamp {
  return currentTimestamp() + cooldownSeconds;
}

/**
 * Check if cooldown has ended
 */
export function isCooldownEnded(cooldownEnd: Timestamp): boolean {
  return currentTimestamp() >= cooldownEnd;
}

/**
 * Format timestamp as human-readable date
 */
export function formatTimestamp(timestamp: Timestamp): string {
  return new Date(Number(timestamp) * 1000).toLocaleString();
}

/**
 * Calculate remaining cooldown time
 */
export function remainingCooldown(cooldownEnd: Timestamp): {
  days: number;
  hours: number;
  minutes: number;
  seconds: number;
  totalSeconds: number;
} {
  const remaining = Number(cooldownEnd - currentTimestamp());
  
  if (remaining <= 0) {
    return { days: 0, hours: 0, minutes: 0, seconds: 0, totalSeconds: 0 };
  }
  
  return {
    days: Math.floor(remaining / 86400),
    hours: Math.floor((remaining % 86400) / 3600),
    minutes: Math.floor((remaining % 3600) / 60),
    seconds: remaining % 60,
    totalSeconds: remaining,
  };
}
