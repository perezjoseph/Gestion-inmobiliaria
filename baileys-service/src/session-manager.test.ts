import { describe, it, expect, beforeAll } from 'vitest';
import * as fc from 'fast-check';
import { encrypt, decrypt } from './pg-auth-state';

describe('Property 1: Session Encryption Round-Trip', () => {
  beforeAll(() => {
    process.env.SESSION_ENCRYPTION_KEY =
      'a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2';
  });

  it('encrypting then decrypting produces the original data for any Buffer input', () => {
    fc.assert(
      fc.property(
        fc.uint8Array({ minLength: 1, maxLength: 4096 }),
        (bytes) => {
          const plaintext = Buffer.from(bytes);
          const ciphertext = encrypt(plaintext);
          const decrypted = decrypt(ciphertext);

          expect(decrypted).toEqual(plaintext);
        }
      ),
      { numRuns: 200 }
    );
  });

  it('ciphertext differs from plaintext for any non-empty Buffer input', () => {
    fc.assert(
      fc.property(
        fc.uint8Array({ minLength: 1, maxLength: 4096 }),
        (bytes) => {
          const plaintext = Buffer.from(bytes);
          const ciphertext = encrypt(plaintext);

          expect(ciphertext.equals(plaintext)).toBe(false);
        }
      ),
      { numRuns: 200 }
    );
  });

  it('ciphertext length is always plaintext + IV (12) + auth tag (16)', () => {
    fc.assert(
      fc.property(
        fc.uint8Array({ minLength: 1, maxLength: 4096 }),
        (bytes) => {
          const plaintext = Buffer.from(bytes);
          const ciphertext = encrypt(plaintext);

          expect(ciphertext.length).toBe(plaintext.length + 12 + 16);
        }
      ),
      { numRuns: 100 }
    );
  });
});
