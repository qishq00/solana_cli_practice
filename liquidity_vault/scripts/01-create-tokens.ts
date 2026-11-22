/**
 * Неделя 11, шаг 1. Создаём два своих тестовых SPL-токена в devnet
 * и минтим их себе на кошелёк.
 *
 *   solana airdrop 2 --url devnet
 *   npm run w11:tokens
 */
import {
  createMint,
  getOrCreateAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";
import { connection, owner, writeState } from "./config";

const DECIMALS = 6;
const SUPPLY = 1_000_000n * 10n ** BigInt(DECIMALS); // 1 млн токенов

async function main() {
  console.log("Кошелёк:", owner.publicKey.toBase58());

  const mintA = await createMint(connection, owner, owner.publicKey, null, DECIMALS);
  const mintB = await createMint(connection, owner, owner.publicKey, null, DECIMALS);

  for (const mint of [mintA, mintB]) {
    const ata = await getOrCreateAssociatedTokenAccount(connection, owner, mint, owner.publicKey);
    await mintTo(connection, owner, mint, ata.address, owner, SUPPLY);
  }

  console.log("VLTA (базовый токен vault):", mintA.toBase58());
  console.log("VLTB (quote-токен):        ", mintB.toBase58());
  writeState({ tokenMint: mintA.toBase58(), quoteMint: mintB.toBase58() });
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
