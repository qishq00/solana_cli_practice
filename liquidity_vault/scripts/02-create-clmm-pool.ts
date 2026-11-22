/**
 * Неделя 11, шаг 2. Создаём CLMM-пул Raydium в devnet для пары VLTA/VLTB
 * и кладём в него стартовую ликвидность в диапазоне ±10% от цены.
 *
 *   npm run w11:pool
 */
import { PoolUtils, TickUtils } from "@raydium-io/raydium-sdk-v2";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import { PublicKey } from "@solana/web3.js";
import BN from "bn.js";
import Decimal from "decimal.js";
import { CLMM_PROGRAM_ID, connection, initSdk, readState, txVersion, writeState } from "./config";

const INITIAL_PRICE = new Decimal(1); // 1 VLTA = 1 VLTB
const DECIMALS = 6;

const tokenInfo = (mint: string, symbol: string) =>
  ({
    address: mint,
    programId: TOKEN_PROGRAM_ID.toBase58(),
    decimals: DECIMALS,
    symbol,
    name: symbol,
    logoURI: "",
    chainId: 101,
    tags: [],
    extensions: {},
  }) as any;

async function main() {
  const state = readState();
  if (!state.tokenMint || !state.quoteMint) throw new Error("Сначала npm run w11:tokens");
  const raydium = await initSdk();

  // 1) Выбираем AMM-конфиг (уровень комиссии + tick spacing)
  const configs = await raydium.api.getClmmConfigs();
  const cfg = configs.sort((a, b) => a.tradeFeeRate - b.tradeFeeRate).find((c) => c.tickSpacing >= 10) ?? configs[0];
  console.log(`AmmConfig ${cfg.id}: fee=${cfg.tradeFeeRate / 1e4}%, tickSpacing=${cfg.tickSpacing}`);

  // 2) Создаём пул
  const { execute, extInfo } = await raydium.clmm.createPool({
    programId: CLMM_PROGRAM_ID,
    mint1: tokenInfo(state.tokenMint, "VLTA"),
    mint2: tokenInfo(state.quoteMint, "VLTB"),
    ammConfig: { ...cfg, id: new PublicKey(cfg.id), fundOwner: "", description: "" },
    initialPrice: INITIAL_PRICE,
    txVersion,
  });
  const { txId } = await execute({ sendAndConfirm: true });
  const poolId = extInfo.address.id.toString();
  console.log("Пул создан:", poolId, "tx:", txId);
  writeState({ poolId });

  // 3) Стартовая ликвидность от кошелька, чтобы через пул можно было обменивать
  const { poolInfo, poolKeys } = await raydium.clmm.getPoolInfoFromRpc(poolId);
  const [lower, upper] = [0.9, 1.1].map(
    (p) =>
      TickUtils.getPriceAndTick({ poolInfo, price: new Decimal(p), baseIn: true }).tick
  );
  const inputAmount = new BN(100_000).mul(new BN(10).pow(new BN(DECIMALS)));
  const epochInfo = await raydium.fetchEpochInfo();
  const res = await PoolUtils.getLiquidityAmountOutFromAmountIn({
    poolInfo,
    slippage: 0,
    inputA: true,
    tickUpper: Math.max(lower, upper),
    tickLower: Math.min(lower, upper),
    amount: inputAmount,
    add: true,
    amountHasFee: true,
    epochInfo,
  });
  const { execute: open } = await raydium.clmm.openPositionFromBase({
    poolInfo,
    poolKeys,
    tickUpper: Math.max(lower, upper),
    tickLower: Math.min(lower, upper),
    base: "MintA",
    ownerInfo: { useSOLBalance: false },
    baseAmount: inputAmount,
    otherAmountMax: res.amountSlippageB.amount.muln(1.05),
    txVersion,
  });
  const { txId: openTx } = await open({ sendAndConfirm: true });
  console.log(`Ликвидность добавлена в диапазон тиков [${lower}, ${upper}], tx:`, openTx);
  console.log("Explorer:", `https://explorer.solana.com/address/${poolId}?cluster=devnet`);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
