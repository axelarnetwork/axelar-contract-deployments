// event-utils.ts
import { PublicKey } from "@solana/web3.js";
import { shake128 } from "@noble/hashes/sha3";
import { bytesToHex as nobleBytesToHex } from "@noble/hashes/utils";
import { Buffer } from "node:buffer";

(BigInt.prototype as any).toJSON = function () {
  return this.toString();
};

export abstract class BaseEvent {
  static readonly EVENT_NAME: string;
  static readonly DISC: Buffer;

  // Abstract static methods don't seem to be supported, let's have this here with a run-time check.
  static deserialize(_segments: Buffer[]): BaseEvent {
    throw new Error("Method not implemented! Use derived class");
  }
}

export type EventClassType = {
  readonly EVENT_NAME: string;
  readonly DISC: Buffer;

  deserialize(segments: Buffer[]): BaseEvent;
};

export type EventParserMap = Map<string, EventClassType>;

export class EventParseError extends Error {
  readonly field?: string | null;
  readonly details?: Record<string, any>;

  constructor(
    message: string,
    field: string | null = null,
    details: Record<string, any> = {}
  ) {
    super(message + (field ? ` (field: ${field})` : ""));
    this.name = this.constructor.name;
    this.field = field;
    this.details = details;

    Object.setPrototypeOf(this, new.target.prototype);
  }
}
export class MissingDataError extends EventParseError {
  constructor(field: string) {
    super("Missing data segment", field);
  }
}
export class InvalidLengthError extends EventParseError {
  constructor(field: string, expected: number, actual: number) {
    super(`Invalid length: expected ${expected}, got ${actual}`, field, {
      expected,
      actual,
    });
  }
}
export class InvalidUtf8Error extends EventParseError {
  constructor(field: string, sourceError?: unknown) {
    super(`Invalid UTF-8`, field, { sourceError });
  }
}
export class TrailingSegmentsError extends EventParseError {
  constructor(count: number) {
    super(`Unexpected trailing data segments found after parsing`, null, {
      count,
    });
  }
}
export class Base64DecodeError extends EventParseError {
  constructor(segmentData?: string, sourceError?: unknown) {
    super(`Base64 decoding failed`, null, { segmentData, sourceError });
  }
}
export class DiscriminantMismatchError extends EventParseError {
  constructor(expectedHex: string, foundHex: string) {
    super(`Discriminant mismatch`, null, { expectedHex, foundHex });
  }
}

export function calculateDiscriminant(eventName: string): Buffer {
  const hash = shake128(Buffer.from(eventName, "utf8"), 16);
  return Buffer.from(hash);
}

export function readPubkey(buffer: Buffer, fieldName: string): PublicKey {
  if (buffer.length !== 32) {
    throw new InvalidLengthError(fieldName, 32, buffer.length);
  }
  return new PublicKey(buffer);
}

export function readFixedU8Array(
  buffer: Buffer,
  length: number,
  fieldName: string
): Buffer {
  if (buffer.length !== length) {
    throw new InvalidLengthError(fieldName, length, buffer.length);
  }
  return buffer;
}

export function readString(buffer: Buffer, fieldName: string): string {
  try {
    return buffer.toString("utf8");
  } catch (e) {
    throw new InvalidUtf8Error(fieldName, e);
  }
}

export function readVecU8(buffer: Buffer, _fieldName: string): Buffer {
  // The entire segment is the Vec<u8> data
  return buffer;
}

export function readU8(buffer: Buffer, fieldName: string): number {
  if (buffer.length !== 1) {
    throw new InvalidLengthError(fieldName, 1, buffer.length);
  }
  return buffer.readUInt8(0);
}

export function readU16LE(buffer: Buffer, fieldName: string): number {
  if (buffer.length !== 2) {
    throw new InvalidLengthError(fieldName, 2, buffer.length);
  }
  return buffer.readUInt16LE(0);
}

export function readU32LE(buffer: Buffer, fieldName: string): number {
  if (buffer.length !== 4) {
    throw new InvalidLengthError(fieldName, 4, buffer.length);
  }
  return buffer.readUInt32LE(0);
}

export function readU64LE(buffer: Buffer, fieldName: string): number {
  if (buffer.length !== 8) {
    throw new InvalidLengthError(fieldName, 8, buffer.length);
  }
  return Number(buffer.readBigUInt64LE(0));
}

export function readU128LE(buffer: Buffer, fieldName: string): bigint {
  if (buffer.length !== 16) {
    throw new InvalidLengthError(fieldName, 16, buffer.length);
  }
  const low = buffer.readBigUInt64LE(0);
  const high = buffer.readBigUInt64LE(8);
  return (high << 64n) | low; // Returns BigInt
}

export function readU256LE(buffer: Buffer, fieldName: string): bigint {
  if (buffer.length !== 32) {
    throw new InvalidLengthError(fieldName, 32, buffer.length);
  }
  // Reconstruct from 4 64-bit chunks
  let result = 0n;
  for (let i = 0; i < 4; i++) {
    result |= buffer.readBigUInt64LE(i * 8) << BigInt(i * 64);
  }
  return result;
}

export function readI8(buffer: Buffer, fieldName: string): number {
  if (buffer.length !== 1)
    throw new InvalidLengthError(fieldName, 1, buffer.length);
  return buffer.readInt8(0);
}
export function readI16LE(buffer: Buffer, fieldName: string): number {
  if (buffer.length !== 2)
    throw new InvalidLengthError(fieldName, 2, buffer.length);
  return buffer.readInt16LE(0);
}
export function readI32LE(buffer: Buffer, fieldName: string): number {
  if (buffer.length !== 4)
    throw new InvalidLengthError(fieldName, 4, buffer.length);
  return buffer.readInt32LE(0);
}
export function readI64LE(buffer: Buffer, fieldName: string): number {
  if (buffer.length !== 8)
    throw new InvalidLengthError(fieldName, 8, buffer.length);
  return Number(buffer.readBigInt64LE(0));
}
export function readI128LE(buffer: Buffer, fieldName: string): bigint {
  if (buffer.length !== 16)
    throw new InvalidLengthError(fieldName, 16, buffer.length);
  const low = buffer.readBigUInt64LE(0); // Read low part as unsigned
  const high = buffer.readBigInt64LE(8); // Read high part as signed
  return (high << 64n) | low; // Returns BigInt
}

export function readBool(buffer: Buffer, fieldName: string): boolean {
  if (buffer.length !== 1) {
    throw new InvalidLengthError(fieldName, 1, buffer.length);
  }
  return buffer.readUInt8(0) !== 0;
}

// Helper to convert discriminant buffer to hex string for map keys/debugging
export function discToHex(buffer: Buffer): string {
  return nobleBytesToHex(buffer);
}

const LOG_PREFIX_ONCHAIN = "Program data: ";

export function decodeLogDataSegments(log: string): Buffer[] | null {
  let dataPart: string | null = null;
  if (log.startsWith(LOG_PREFIX_ONCHAIN)) {
    dataPart = log.substring(LOG_PREFIX_ONCHAIN.length);
  } else {
    return null; // Not a recognized data log line
  }

  const segments = dataPart.split(" ");
  if (segments.length === 0) {
    return []; // Return empty array for empty data part
  }

  try {
    return segments.map((s) => Buffer.from(s, "base64"));
  } catch (e) {
    throw new Base64DecodeError(dataPart, e); // Throw specific error
  }
}

// Interface for parsing options
interface ParseLogsOptions {
  /** If true, logs parsing errors to console and continues. If false, throws
   * the first parsing error. Defaults to true. */
  ignoreErrors?: boolean;
}

export function parseEventsFromLogs(
  logs: string[],
  eventParserMap: EventParserMap,
  options: ParseLogsOptions = { ignoreErrors: true }
): BaseEvent[] {
  const parsedEvents: BaseEvent[] = [];

  for (const log of logs) {
    let decodedSlices: Buffer[] | null = null;
    try {
      decodedSlices = decodeLogDataSegments(log);
      if (decodedSlices === null) {
        continue; // Not a data log line we recognize
      }

      if (decodedSlices.length === 0) {
        if (!options.ignoreErrors) throw new MissingDataError("discriminant");
        console.warn(
          `Skipping log: No data segments found after prefix. Log: "${log}"`
        );
        continue;
      }

      // First segment is the discriminant, pop it off
      const discriminant = decodedSlices.shift();
      if (!discriminant) {
        // Should not happen if decodedSlices.length > 0, but satisfy TS
        if (!options.ignoreErrors)
          throw new MissingDataError("discriminant (internal error)");
        console.warn(
          `Skipping log: Internal error getting discriminant. Log: "${log}"`
        );
        continue;
      }

      if (discriminant.length !== 16) {
        continue;
      }

      const discHex = discToHex(discriminant);
      const EventClass = eventParserMap.get(discHex);

      if (EventClass) {
        const eventObject = EventClass.deserialize(decodedSlices);
        parsedEvents.push(eventObject);
      }
    } catch (error: unknown) {
      if (!options.ignoreErrors) {
        throw error as EventParseError;
      } else {
        if (error instanceof EventParseError) {
          console.warn(
            `Error parsing event log: ${error.message}. Log: "${log}"`,
            error.details ?? ""
          );
        } else if (error instanceof Error) {
          console.warn(
            `Unexpected error parsing event log: ${error.message}. Log: "${log}"`,
            error.stack
          );
        } else {
          console.warn(
            `Unexpected non-error thrown during parsing. Log: "${log}"`,
            error
          );
        }
      }
    }
  }

  return parsedEvents;
}
