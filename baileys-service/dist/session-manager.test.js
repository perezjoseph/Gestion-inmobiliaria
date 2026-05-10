"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
const vitest_1 = require("vitest");
const fc = __importStar(require("fast-check"));
const pg_auth_state_1 = require("./pg-auth-state");
/**
 * Property 1: Session Encryption Round-Trip
 *
 * For any session authentication state data, encrypting with AES-256-GCM
 * then decrypting with the same key SHALL produce the original data,
 * and the ciphertext SHALL differ from the plaintext.
 *
 * **Validates: Requirements 1.6**
 */
(0, vitest_1.describe)('Property 1: Session Encryption Round-Trip', () => {
    (0, vitest_1.beforeAll)(() => {
        // Set a valid 64 hex character encryption key for testing
        process.env.SESSION_ENCRYPTION_KEY =
            'a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4a5b6c7d8e9f0a1b2';
    });
    (0, vitest_1.it)('encrypting then decrypting produces the original data for any Buffer input', () => {
        fc.assert(fc.property(fc.uint8Array({ minLength: 1, maxLength: 4096 }), (bytes) => {
            const plaintext = Buffer.from(bytes);
            const ciphertext = (0, pg_auth_state_1.encrypt)(plaintext);
            const decrypted = (0, pg_auth_state_1.decrypt)(ciphertext);
            // Round-trip: decrypt(encrypt(data)) === data
            (0, vitest_1.expect)(decrypted).toEqual(plaintext);
        }), { numRuns: 200 });
    });
    (0, vitest_1.it)('ciphertext differs from plaintext for any non-empty Buffer input', () => {
        fc.assert(fc.property(fc.uint8Array({ minLength: 1, maxLength: 4096 }), (bytes) => {
            const plaintext = Buffer.from(bytes);
            const ciphertext = (0, pg_auth_state_1.encrypt)(plaintext);
            // Ciphertext must differ from plaintext
            (0, vitest_1.expect)(ciphertext.equals(plaintext)).toBe(false);
        }), { numRuns: 200 });
    });
    (0, vitest_1.it)('ciphertext length is always plaintext + IV (12) + auth tag (16)', () => {
        fc.assert(fc.property(fc.uint8Array({ minLength: 1, maxLength: 4096 }), (bytes) => {
            const plaintext = Buffer.from(bytes);
            const ciphertext = (0, pg_auth_state_1.encrypt)(plaintext);
            // AES-256-GCM output: IV (12) + AuthTag (16) + ciphertext (same length as plaintext)
            (0, vitest_1.expect)(ciphertext.length).toBe(plaintext.length + 12 + 16);
        }), { numRuns: 100 });
    });
});
//# sourceMappingURL=session-manager.test.js.map