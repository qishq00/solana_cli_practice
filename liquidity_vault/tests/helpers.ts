import { PublicKey } from "@solana/web3.js";

export const seed = (s: string) => Buffer.from(s);

export function vaultPdas(programId: PublicKey, tokenMint: PublicKey) {
  const [config] = PublicKey.findProgramAddressSync(
    [seed("vault"), tokenMint.toBuffer()],
    programId
  );
  const [vaultAuthority] = PublicKey.findProgramAddressSync(
    [seed("authority"), config.toBuffer()],
    programId
  );
  const [shareMint] = PublicKey.findProgramAddressSync(
    [seed("share_mint"), config.toBuffer()],
    programId
  );
  const [tokenVault] = PublicKey.findProgramAddressSync(
    [seed("token_vault"), config.toBuffer()],
    programId
  );
  return { config, vaultAuthority, shareMint, tokenVault };
}

export function userPda(
  programId: PublicKey,
  config: PublicKey,
  owner: PublicKey
) {
  return PublicKey.findProgramAddressSync(
    [seed("user"), config.toBuffer(), owner.toBuffer()],
    programId
  )[0];
}
