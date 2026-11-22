/**
 * Общие настройки скриптов: devnet-подключение, кошелёк, Raydium SDK,
 * Anchor-программа vault и файл состояния scripts/state.json.
 */
import * as anchor from "@coral-xyz/anchor";
import { Raydium, TxVersion, DEVNET_PROGRAM_ID } from "@raydium-io/raydium-sdk-v2";
import { clusterApiUrl, Connection, Keypair, PublicKey } from "@solana/web3.js";
import fs from "fs";
import os from "os";
import path from "path";

export const connection = new Connection(
  process.env.RPC_URL ?? clusterApiUrl("devnet"),
  "confirmed"
);

export const owner: Keypair = Keypair.fromSecretKey(
  Uint8Array.from(
    JSON.parse(
      fs.readFileSync(
        process.env.WALLET ?? path.join(os.homedir(), ".config/solana/id.json"),
        "utf8"
      )
    )
  )
);

export const txVersion = TxVersion.V0;

// В разных версиях SDK поле называется CLMM_PROGRAM_ID или CLMM.
export const CLMM_PROGRAM_ID: PublicKey =
  (DEVNET_PROGRAM_ID as any).CLMM_PROGRAM_ID ?? (DEVNET_PROGRAM_ID as any).CLMM;

let raydium: Raydium | undefined;
export async function initSdk(): Promise<Raydium> {
  if (raydium) return raydium;
  raydium = await Raydium.load({
    owner,
    connection,
    cluster: "devnet",
    disableFeatureCheck: true,
    disableLoadToken: true,
    blockhashCommitment: "finalized",
  });
  return raydium;
}

export function loadProgram(): anchor.Program {
  const provider = new anchor.AnchorProvider(connection, new anchor.Wallet(owner), {
    commitment: "confirmed",
  });
  anchor.setProvider(provider);
  const idl = JSON.parse(
    fs.readFileSync(path.join(__dirname, "../target/idl/liquidity_vault.json"), "utf8")
  );
  return new anchor.Program(idl, provider);
}

// ---------- состояние между скриптами ----------
const STATE_FILE = path.join(__dirname, "state.json");

export type State = {
  tokenMint?: string; // базовый токен vault (VLTA)
  quoteMint?: string; // второй токен пары (VLTB)
  poolId?: string;
  positionNftMint?: string;
};

export function readState(): State {
  return fs.existsSync(STATE_FILE) ? JSON.parse(fs.readFileSync(STATE_FILE, "utf8")) : {};
}

export function writeState(patch: State) {
  const next = { ...readState(), ...patch };
  fs.writeFileSync(STATE_FILE, JSON.stringify(next, null, 2));
  return next;
}

export const seed = (s: string) => Buffer.from(s);

export function vaultPdas(programId: PublicKey, tokenMint: PublicKey) {
  const [config] = PublicKey.findProgramAddressSync([seed("vault"), tokenMint.toBuffer()], programId);
  const pda = (s: string) =>
    PublicKey.findProgramAddressSync([seed(s), config.toBuffer()], programId)[0];
  return {
    config,
    vaultAuthority: pda("authority"),
    shareMint: pda("share_mint"),
    tokenVault: pda("token_vault"),
    quoteVault: pda("quote_vault"),
  };
}
