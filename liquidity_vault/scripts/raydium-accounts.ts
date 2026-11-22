/**
 * Чтение PoolState и расчёт PDA Raydium CLMM для передачи в инструкции vault.
 */
import { PoolInfoLayout, getPdaExBitmapAccount } from "@raydium-io/raydium-sdk-v2";
import { PublicKey } from "@solana/web3.js";
import { CLMM_PROGRAM_ID, connection } from "./config";

export const TICK_ARRAY_SIZE = 60;
export const MEMO_PROGRAM_ID = new PublicKey("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");

export async function fetchPool(poolId: PublicKey) {
  const info = await connection.getAccountInfo(poolId);
  if (!info) throw new Error("Пул не найден");
  return PoolInfoLayout.decode(info.data);
}

export function bitmapExtension(poolId: PublicKey) {
  return getPdaExBitmapAccount(CLMM_PROGRAM_ID, poolId).publicKey;
}

const i32be = (n: number) => {
  const b = Buffer.alloc(4);
  b.writeInt32BE(n);
  return b;
};

export function tickArrayStart(tick: number, tickSpacing: number) {
  const span = tickSpacing * TICK_ARRAY_SIZE;
  return Math.floor(tick / span) * span;
}

export function tickArrayPda(poolId: PublicKey, startIndex: number) {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("tick_array"), poolId.toBuffer(), i32be(startIndex)],
    CLMM_PROGRAM_ID
  )[0];
}

export function protocolPositionPda(poolId: PublicKey, lowerStart: number, upperStart: number) {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("position"), poolId.toBuffer(), i32be(lowerStart), i32be(upperStart)],
    CLMM_PROGRAM_ID
  )[0];
}

export function personalPositionPda(nftMint: PublicKey) {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("position"), nftMint.toBuffer()],
    CLMM_PROGRAM_ID
  )[0];
}
