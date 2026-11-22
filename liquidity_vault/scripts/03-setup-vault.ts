/**
 * Неделя 11, шаг 3. Инициализируем vault для VLTA, хранилище VLTB
 * и привязываем vault к созданному CLMM-пулу.
 *
 *   anchor build && anchor deploy --provider.cluster devnet
 *   npm run w11:vault
 */
import { BN } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { loadProgram, owner, readState, vaultPdas } from "./config";

async function main() {
  const state = readState();
  if (!state.tokenMint || !state.quoteMint || !state.poolId) {
    throw new Error("Сначала w11:tokens и w11:pool");
  }
  const program = loadProgram();
  const tokenMint = new PublicKey(state.tokenMint);
  const pdas = vaultPdas(program.programId, tokenMint);

  if (!(await program.provider.connection.getAccountInfo(pdas.config))) {
    await program.methods.initialize(new BN(0)).accounts({ admin: owner.publicKey, tokenMint }).rpc();
    console.log("Vault создан:", pdas.config.toBase58());
  }

  if (!(await program.provider.connection.getAccountInfo(pdas.quoteVault))) {
    await program.methods
      .initQuoteVault()
      .accounts({ admin: owner.publicKey, config: pdas.config, quoteMint: new PublicKey(state.quoteMint) })
      .rpc();
    console.log("Quote-хранилище:", pdas.quoteVault.toBase58());
  }

  await program.methods
    .setPool()
    .accounts({ admin: owner.publicKey, config: pdas.config, poolState: new PublicKey(state.poolId) })
    .rpc();
  const cfg: any = await (program.account as any).vaultConfig.fetch(pdas.config);
  console.log("Vault привязан к пулу:", cfg.clmmPool.toBase58());
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
