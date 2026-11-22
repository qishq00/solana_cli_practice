/**
 * Неделя 13. Позиция vault в CLMM-пуле Raydium.
 *
 *   npm run w13 -- open [ширина_%=5]   открыть позицию в диапазоне ±ширина от текущей цены
 *   npm run w13 -- status              показать диапазон, текущий тик и комиссии
 *   npm run w13 -- fees                собрать комиссии в хранилища vault
 *   npm run w13 -- push-price [сумма]  (демо) сдвинуть цену крупным swap-ом с кошелька
 *   npm run w13 -- rebalance [ширина]  если цена вне диапазона: вывести ликвидность,
 *                                      закрыть позицию, выровнять токены, открыть новую
 *   npm run w13 -- close               вывести всю ликвидность и закрыть позицию
 */
import { BN } from "@coral-xyz/anchor";
import {
  LiquidityMath,
  PoolUtils,
  PositionInfoLayout,
  SqrtPriceMath,
} from "@raydium-io/raydium-sdk-v2";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  getAccount,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import {
  CLMM_PROGRAM_ID,
  connection,
  initSdk,
  loadProgram,
  owner,
  readState,
  txVersion,
  vaultPdas,
  writeState,
} from "./config";
import {
  MEMO_PROGRAM_ID,
  bitmapExtension,
  fetchPool,
  personalPositionPda,
  protocolPositionPda,
  tickArrayPda,
  tickArrayStart,
} from "./raydium-accounts";

const METADATA_PROGRAM_ID = new PublicKey("metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s");

const state = readState();
const program = loadProgram();
const methods = program.methods as any;
const tokenMint = new PublicKey(state.tokenMint!);
const poolId = new PublicKey(state.poolId!);
const pdas = vaultPdas(program.programId, tokenMint);

const fetchConfig = () => (program.account as any).vaultConfig.fetch(pdas.config);
const balance = async (acc: PublicKey) => new BN((await getAccount(connection, acc)).amount.toString());

/** Хранилища vault в порядке токенов пула (mint0, mint1). */
async function vaultAccountsInPoolOrder() {
  const pool = await fetchPool(poolId);
  const tokenIs0 = pool.mintA.equals(tokenMint);
  return {
    pool,
    acc0: tokenIs0 ? pdas.tokenVault : pdas.quoteVault,
    acc1: tokenIs0 ? pdas.quoteVault : pdas.tokenVault,
  };
}

function rangeAround(tickCurrent: number, spacing: number, widthPct: number) {
  const w = Math.round(Math.log(1 + widthPct / 100) / Math.log(1.0001));
  const lower = Math.floor((tickCurrent - w) / spacing) * spacing;
  const upper = Math.ceil((tickCurrent + w) / spacing) * spacing;
  return { lower, upper: upper === lower ? upper + spacing : upper };
}

/** vault_authority платит ренту за аккаунты позиции — пополняем его SOL. */
async function fundAuthority(minSol = 0.05) {
  const lamports = await connection.getBalance(pdas.vaultAuthority);
  if (lamports >= minSol * LAMPORTS_PER_SOL) return;
  const tx = new Transaction().add(
    SystemProgram.transfer({
      fromPubkey: owner.publicKey,
      toPubkey: pdas.vaultAuthority,
      lamports: Math.ceil(minSol * LAMPORTS_PER_SOL) - lamports,
    })
  );
  await sendAndConfirmTransaction(connection, tx, [owner]);
  console.log(`vault_authority пополнен до ${minSol} SOL`);
}

// ---------------------------------------------------------------------
async function openPosition(widthPct: number) {
  await fundAuthority();
  const { pool, acc0, acc1 } = await vaultAccountsInPoolOrder();
  const { lower, upper } = rangeAround(pool.tickCurrent, pool.tickSpacing, widthPct);
  const lowerStart = tickArrayStart(lower, pool.tickSpacing);
  const upperStart = tickArrayStart(upper, pool.tickSpacing);

  // Ликвидность под балансы vault (берём 95%, запас на округление)
  const amount0 = (await balance(acc0)).muln(95).divn(100);
  const amount1 = (await balance(acc1)).muln(95).divn(100);
  const liquidity = LiquidityMath.getLiquidityFromTokenAmounts(
    pool.sqrtPriceX64,
    SqrtPriceMath.getSqrtPriceX64FromTick(lower),
    SqrtPriceMath.getSqrtPriceX64FromTick(upper),
    amount0,
    amount1
  );
  if (liquidity.isZero()) throw new Error("В vault нет токенов для позиции (сначала w12:swap)");

  const nftMint = Keypair.generate();
  const [metadata] = PublicKey.findProgramAddressSync(
    [Buffer.from("metadata"), METADATA_PROGRAM_ID.toBuffer(), nftMint.publicKey.toBuffer()],
    METADATA_PROGRAM_ID
  );

  const sig = await methods
    .openPosition(lower, upper, lowerStart, upperStart, liquidity, amount0, amount1)
    .accounts({
      admin: owner.publicKey,
      config: pdas.config,
      vaultAuthority: pdas.vaultAuthority,
      clmmProgram: CLMM_PROGRAM_ID,
      positionNftMint: nftMint.publicKey,
      positionNftAccount: getAssociatedTokenAddressSync(nftMint.publicKey, pdas.vaultAuthority, true),
      metadataAccount: metadata,
      poolState: poolId,
      protocolPosition: protocolPositionPda(poolId, lowerStart, upperStart),
      tickArrayLower: tickArrayPda(poolId, lowerStart),
      tickArrayUpper: tickArrayPda(poolId, upperStart),
      personalPosition: personalPositionPda(nftMint.publicKey),
      tokenAccount0: acc0,
      tokenAccount1: acc1,
      tokenVault0: pool.vaultA,
      tokenVault1: pool.vaultB,
      vault0Mint: pool.mintA,
      vault1Mint: pool.mintB,
      tokenProgram: TOKEN_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      metadataProgram: METADATA_PROGRAM_ID,
      tokenProgram2022: TOKEN_2022_PROGRAM_ID,
    })
    .remainingAccounts([{ pubkey: bitmapExtension(poolId), isSigner: false, isWritable: true }])
    .signers([nftMint])
    .rpc();

  writeState({ positionNftMint: nftMint.publicKey.toBase58() });
  console.log(`Позиция открыта: тики [${lower}, ${upper}), текущий ${pool.tickCurrent}, L=${liquidity}`);
  console.log("tx:", `https://explorer.solana.com/tx/${sig}?cluster=devnet`);
}

// ---------------------------------------------------------------------
async function modifyAccounts() {
  const cfg = await fetchConfig();
  const { pool, acc0, acc1 } = await vaultAccountsInPoolOrder();
  const nftMint: PublicKey = cfg.positionNftMint;
  const lowerStart = tickArrayStart(cfg.positionTickLower, pool.tickSpacing);
  const upperStart = tickArrayStart(cfg.positionTickUpper, pool.tickSpacing);
  return {
    cfg,
    nftMint,
    accounts: {
      admin: owner.publicKey,
      config: pdas.config,
      vaultAuthority: pdas.vaultAuthority,
      clmmProgram: CLMM_PROGRAM_ID,
      positionNftAccount: getAssociatedTokenAddressSync(nftMint, pdas.vaultAuthority, true),
      personalPosition: personalPositionPda(nftMint),
      poolState: poolId,
      protocolPosition: protocolPositionPda(poolId, lowerStart, upperStart),
      tokenVault0: pool.vaultA,
      tokenVault1: pool.vaultB,
      tickArrayLower: tickArrayPda(poolId, lowerStart),
      tickArrayUpper: tickArrayPda(poolId, upperStart),
      recipientTokenAccount0: acc0,
      recipientTokenAccount1: acc1,
      tokenProgram: TOKEN_PROGRAM_ID,
      tokenProgram2022: TOKEN_2022_PROGRAM_ID,
      memoProgram: MEMO_PROGRAM_ID,
      vault0Mint: pool.mintA,
      vault1Mint: pool.mintB,
    },
    remaining: [{ pubkey: bitmapExtension(poolId), isSigner: false, isWritable: true }],
  };
}

async function positionInfo(nftMint: PublicKey) {
  const info = await connection.getAccountInfo(personalPositionPda(nftMint));
  if (!info) throw new Error("Позиция не найдена");
  return PositionInfoLayout.decode(info.data);
}

async function status() {
  const cfg = await fetchConfig();
  const pool = await fetchPool(poolId);
  if (cfg.positionNftMint.equals(PublicKey.default)) {
    console.log("Позиции нет. Текущий тик:", pool.tickCurrent);
    return;
  }
  const pos = await positionInfo(cfg.positionNftMint);
  const inRange = pool.tickCurrent >= cfg.positionTickLower && pool.tickCurrent < cfg.positionTickUpper;
  console.log(`Диапазон [${cfg.positionTickLower}, ${cfg.positionTickUpper}), текущий тик ${pool.tickCurrent}`);
  console.log(inRange ? "✅ цена в диапазоне — позиция зарабатывает" : "⚠️ цена вне диапазона — нужна перестановка");
  console.log(`L=${pos.liquidity}, комиссии к выводу (зафиксированные): ${pos.tokenFeesOwedA} / ${pos.tokenFeesOwedB}`);
}

async function collectFees() {
  const { accounts, remaining } = await modifyAccounts();
  const before = [await balance(accounts.recipientTokenAccount0), await balance(accounts.recipientTokenAccount1)];
  const sig = await methods.collectFees().accounts(accounts).remainingAccounts(remaining).rpc();
  const after = [await balance(accounts.recipientTokenAccount0), await balance(accounts.recipientTokenAccount1)];
  console.log(`Комиссии собраны: token0 +${after[0].sub(before[0])}, token1 +${after[1].sub(before[1])}`);
  console.log("tx:", `https://explorer.solana.com/tx/${sig}?cluster=devnet`);
}

async function withdrawAllAndClose() {
  const { cfg, nftMint, accounts, remaining } = await modifyAccounts();
  const pos = await positionInfo(nftMint);
  // Вся ликвидность + комиссии; минимумы 0 — выводим себе же, сэндвич невыгоден,
  // но в проде здесь тоже нужен min по текущей цене.
  await methods
    .decreaseLiquidity(pos.liquidity, new BN(0), new BN(0))
    .accounts(accounts)
    .remainingAccounts(remaining)
    .rpc();
  console.log(`Ликвидность выведена: L=${pos.liquidity}`);

  const sig = await methods
    .closePosition()
    .accounts({
      admin: owner.publicKey,
      config: pdas.config,
      vaultAuthority: pdas.vaultAuthority,
      clmmProgram: CLMM_PROGRAM_ID,
      positionNftMint: nftMint,
      positionNftAccount: accounts.positionNftAccount,
      personalPosition: accounts.personalPosition,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .rpc();
  console.log(`Позиция ${cfg.positionNftMint.toBase58()} закрыта, tx:`, sig);
}

/** Выравниваем токены vault ~50/50 по стоимости перед новой позицией (через swap недели 12). */
async function rebalanceTokens() {
  const raydium = await initSdk();
  const { poolInfo, computePoolInfo, tickData } = await raydium.clmm.getPoolInfoFromRpc(poolId.toBase58());
  const pool = await fetchPool(poolId);
  const tokenIs0 = pool.mintA.equals(tokenMint);
  const base = await balance(pdas.tokenVault);
  const quoteBal = await balance(pdas.quoteVault);

  // цена token1 за token0 (decimals одинаковые)
  const price = Number(poolInfo.price);
  const baseValue = Number(base) * (tokenIs0 ? price : 1 / price); // в единицах quote
  const quoteValue = Number(quoteBal);
  const baseToQuote = baseValue > quoteValue;
  const excessInQuote = Math.abs(baseValue - quoteValue) / 2;
  const amountIn = new BN(Math.floor(baseToQuote ? excessInQuote / (tokenIs0 ? price : 1 / price) : excessInQuote));
  if (amountIn.isZero()) return;

  const inputIsMintA = baseToQuote === tokenIs0;
  const q = PoolUtils.computeAmountOutFormat({
    poolInfo: computePoolInfo,
    tickArrayCache: tickData[poolId.toBase58()],
    amountIn,
    tokenOut: inputIsMintA ? poolInfo.mintB : poolInfo.mintA,
    slippage: 0.01,
    epochInfo: await raydium.fetchEpochInfo(),
  });
  await methods
    .swap(amountIn, new BN(q.minAmountOut.amount.raw.toString()), new BN(0), baseToQuote)
    .accounts({
      admin: owner.publicKey,
      config: pdas.config,
      clmmProgram: CLMM_PROGRAM_ID,
      ammConfig: pool.ammConfig,
      poolState: poolId,
      poolInputVault: inputIsMintA ? pool.vaultA : pool.vaultB,
      poolOutputVault: inputIsMintA ? pool.vaultB : pool.vaultA,
      observationState: pool.observationId,
      inputMint: inputIsMintA ? pool.mintA : pool.mintB,
      outputMint: inputIsMintA ? pool.mintB : pool.mintA,
      tokenProgram2022: TOKEN_2022_PROGRAM_ID,
      memoProgram: MEMO_PROGRAM_ID,
    })
    .remainingAccounts([
      { pubkey: bitmapExtension(poolId), isSigner: false, isWritable: true },
      ...q.remainingAccounts.map((pubkey: PublicKey) => ({ pubkey, isSigner: false, isWritable: true })),
    ])
    .rpc();
  console.log(`Выравнивание: swap ${amountIn} (${baseToQuote ? "base→quote" : "quote→base"})`);
}

async function rebalance(widthPct: number) {
  try {
    await methods.checkRebalance().accounts({ config: pdas.config, poolState: poolId }).simulate();
  } catch (e: any) {
    if (String(e?.message ?? e).includes("PositionInRange") || JSON.stringify(e?.simulationResponse ?? "").includes("PositionInRange")) {
      console.log("Цена в диапазоне — перестановка не нужна.");
      return;
    }
    throw e;
  }
  console.log("Цена вышла из диапазона → переставляем позицию");
  await collectFees();
  await withdrawAllAndClose();
  await rebalanceTokens();
  await openPosition(widthPct);
}

/** Демо: крупный swap с кошелька (не из vault), чтобы цена вышла из диапазона. */
async function pushPrice(amount: number) {
  const raydium = await initSdk();
  const { poolInfo, poolKeys, computePoolInfo, tickData } = await raydium.clmm.getPoolInfoFromRpc(poolId.toBase58());
  const amountIn = new BN(amount).mul(new BN(10).pow(new BN(6)));
  const { minAmountOut, remainingAccounts } = PoolUtils.computeAmountOutFormat({
    poolInfo: computePoolInfo,
    tickArrayCache: tickData[poolId.toBase58()],
    amountIn,
    tokenOut: poolInfo.mintB,
    slippage: 0.5,
    epochInfo: await raydium.fetchEpochInfo(),
  });
  const { execute } = await raydium.clmm.swap({
    poolInfo,
    poolKeys,
    inputMint: poolInfo.mintA.address,
    amountIn,
    amountOutMin: minAmountOut.amount.raw,
    observationId: computePoolInfo.observationId,
    ownerInfo: { useSOLBalance: false },
    remainingAccounts,
    txVersion,
  });
  const { txId } = await execute({ sendAndConfirm: true });
  console.log("Цена сдвинута, tx:", txId);
  await status();
}

async function main() {
  const [cmd = "status", arg] = process.argv.slice(2);
  switch (cmd) {
    case "open":
      return openPosition(Number(arg ?? 5));
    case "status":
      return status();
    case "fees":
      return collectFees();
    case "push-price":
      return pushPrice(Number(arg ?? 50_000));
    case "rebalance":
      return rebalance(Number(arg ?? 5));
    case "close":
      return withdrawAllAndClose();
    default:
      throw new Error(`Неизвестная команда: ${cmd}`);
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
