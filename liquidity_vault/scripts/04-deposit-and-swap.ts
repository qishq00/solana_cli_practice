/**
 * Неделя 12. Пользователь вносит VLTA в vault, затем vault меняет часть VLTA
 * на VLTB через CLMM-пул Raydium с ограничением минимальной суммы получения.
 *
 *   npm run w12:swap -- [сумма_обмена=30000] [slippage_%=1]
 */
import { BN } from "@coral-xyz/anchor";
import { PoolUtils } from "@raydium-io/raydium-sdk-v2";
import { getAccount, getOrCreateAssociatedTokenAccount, TOKEN_2022_PROGRAM_ID } from "@solana/spl-token";
import { PublicKey } from "@solana/web3.js";
import { CLMM_PROGRAM_ID, initSdk, loadProgram, owner, readState, vaultPdas } from "./config";
import { bitmapExtension, fetchPool, MEMO_PROGRAM_ID } from "./raydium-accounts";

const DECIMALS = 6;
const toUnits = (n: number) => new BN(Math.round(n * 10 ** DECIMALS));

async function main() {
  const swapAmount = Number(process.argv[2] ?? 30_000);
  const slippage = Number(process.argv[3] ?? 1) / 100;

  const state = readState();
  const program = loadProgram();
  const conn = program.provider.connection;
  const tokenMint = new PublicKey(state.tokenMint!);
  const quoteMint = new PublicKey(state.quoteMint!);
  const poolId = new PublicKey(state.poolId!);
  const pdas = vaultPdas(program.programId, tokenMint);

  // 1) Депозит пользователя (неделя 9)
  const [userPos] = PublicKey.findProgramAddressSync(
    [Buffer.from("user"), pdas.config.toBuffer(), owner.publicKey.toBuffer()],
    program.programId
  );
  if (!(await conn.getAccountInfo(userPos))) {
    await program.methods.initUser().accounts({ owner: owner.publicKey, config: pdas.config }).rpc();
  }
  const userToken = await getOrCreateAssociatedTokenAccount(conn, owner, tokenMint, owner.publicKey);
  const userShare = await getOrCreateAssociatedTokenAccount(conn, owner, pdas.shareMint, owner.publicKey);
  await program.methods
    .deposit(toUnits(100_000))
    .accounts({
      owner: owner.publicKey,
      config: pdas.config,
      userTokenAccount: userToken.address,
      userShareAccount: userShare.address,
    })
    .rpc();
  console.log("Внесено в vault: 100 000 VLTA");

  // 2) Котировка через Raydium SDK: ожидаемый выход, минимум с учётом slippage,
  //    и список tick array-аккаунтов, через которые пройдёт обмен.
  const raydium = await initSdk();
  const { poolInfo, computePoolInfo, tickData } = await raydium.clmm.getPoolInfoFromRpc(poolId.toBase58());
  const baseIsMintA = poolInfo.mintA.address === tokenMint.toBase58();
  const amountIn = toUnits(swapAmount);
  const quote = PoolUtils.computeAmountOutFormat({
    poolInfo: computePoolInfo,
    tickArrayCache: tickData[poolId.toBase58()],
    amountIn,
    tokenOut: baseIsMintA ? poolInfo.mintB : poolInfo.mintA,
    slippage,
    epochInfo: await raydium.fetchEpochInfo(),
  });
  const minOut = new BN(quote.minAmountOut.amount.raw.toString());
  console.log(
    `Котировка: ${swapAmount} VLTA → ~${quote.amountOut.amount.toExact()} VLTB, ` +
      `минимум ${quote.minAmountOut.amount.toExact()} (slippage ${slippage * 100}%)`
  );

  // 3) Swap из vault (неделя 12)
  const pool = await fetchPool(poolId);
  const tokenIs0 = pool.mintA.equals(tokenMint);
  const remaining = [
    { pubkey: bitmapExtension(poolId), isSigner: false, isWritable: true },
    ...quote.remainingAccounts.map((pubkey: PublicKey) => ({ pubkey, isSigner: false, isWritable: true })),
  ];

  const sig = await program.methods
    .swap(amountIn, minOut, new BN(0), true)
    .accounts({
      admin: owner.publicKey,
      config: pdas.config,
      clmmProgram: CLMM_PROGRAM_ID,
      ammConfig: pool.ammConfig,
      poolState: poolId,
      poolInputVault: tokenIs0 ? pool.vaultA : pool.vaultB,
      poolOutputVault: tokenIs0 ? pool.vaultB : pool.vaultA,
      observationState: pool.observationId,
      inputMint: tokenMint,
      outputMint: quoteMint,
      tokenProgram2022: TOKEN_2022_PROGRAM_ID,
      memoProgram: MEMO_PROGRAM_ID,
    })
    .remainingAccounts(remaining)
    .rpc();

  const base = await getAccount(conn, pdas.tokenVault);
  const quoteBal = await getAccount(conn, pdas.quoteVault);
  console.log("Swap tx:", `https://explorer.solana.com/tx/${sig}?cluster=devnet`);
  console.log(`Баланс vault: VLTA=${Number(base.amount) / 1e6}, VLTB=${Number(quoteBal.amount) / 1e6}`);

  // 4) Демонстрация защиты: заведомо невыполнимый минимум → транзакция откатывается
  try {
    await program.methods
      .swap(toUnits(1), toUnits(1_000), new BN(0), true)
      .accounts({
        admin: owner.publicKey,
        config: pdas.config,
        clmmProgram: CLMM_PROGRAM_ID,
        ammConfig: pool.ammConfig,
        poolState: poolId,
        poolInputVault: tokenIs0 ? pool.vaultA : pool.vaultB,
        poolOutputVault: tokenIs0 ? pool.vaultB : pool.vaultA,
        observationState: pool.observationId,
        inputMint: tokenMint,
        outputMint: quoteMint,
        tokenProgram2022: TOKEN_2022_PROGRAM_ID,
        memoProgram: MEMO_PROGRAM_ID,
      })
      .remainingAccounts(remaining)
      .rpc();
  } catch (e: any) {
    console.log("Ожидаемо отклонено (slippage):", String(e.message ?? e).split("\n")[0]);
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
