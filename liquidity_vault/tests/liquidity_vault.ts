import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import {
  createMint,
  getMint,
  getAccount,
  getOrCreateAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";
import { expect } from "chai";
import { LiquidityVault } from "../target/types/liquidity_vault";
import { vaultPdas, userPda } from "./helpers";

describe("liquidity_vault", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.LiquidityVault as Program<LiquidityVault>;
  const admin = (provider.wallet as anchor.Wallet).payer;

  let tokenMint: anchor.web3.PublicKey;
  let pdas: ReturnType<typeof vaultPdas>;
  let userToken: anchor.web3.PublicKey;
  let userShare: anchor.web3.PublicKey;

  // ---------- Неделя 8: каркас ----------
  it("initialize: создаёт config, share-mint и хранилище", async () => {
    tokenMint = await createMint(provider.connection, admin, admin.publicKey, null, 6);
    pdas = vaultPdas(program.programId, tokenMint);

    await program.methods
      .initialize(new BN(0))
      .accounts({ admin: admin.publicKey, tokenMint })
      .rpc();

    const config = await program.account.vaultConfig.fetch(pdas.config);
    expect(config.admin.toBase58()).to.eq(admin.publicKey.toBase58());
    expect(config.shareMint.toBase58()).to.eq(pdas.shareMint.toBase58());

    const share = await getMint(provider.connection, pdas.shareMint);
    expect(share.decimals).to.eq(6);
    expect(share.mintAuthority!.toBase58()).to.eq(pdas.vaultAuthority.toBase58());
  });

  it("init_user: создаёт PDA пользователя", async () => {
    await program.methods
      .initUser()
      .accounts({ owner: admin.publicKey, config: pdas.config })
      .rpc();
    const pos = await program.account.userPosition.fetch(
      userPda(program.programId, pdas.config, admin.publicKey)
    );
    expect(pos.shares.toNumber()).to.eq(0);
  });

  it("update_config: только админ", async () => {
    const stranger = anchor.web3.Keypair.generate();
    try {
      await program.methods
        .updateConfig(true, null)
        .accounts({ admin: stranger.publicKey, config: pdas.config })
        .signers([stranger])
        .rpc();
      expect.fail("должно было упасть");
    } catch (e: any) {
      expect(String(e)).to.match(/Unauthorized|ConstraintHasOne|Error/);
    }
  });

  // ---------- Неделя 9: депозит ----------
  it("deposit: первый депозит 1:1", async () => {
    userToken = (
      await getOrCreateAssociatedTokenAccount(provider.connection, admin, tokenMint, admin.publicKey)
    ).address;
    userShare = (
      await getOrCreateAssociatedTokenAccount(provider.connection, admin, pdas.shareMint, admin.publicKey)
    ).address;
    await mintTo(provider.connection, admin, tokenMint, userToken, admin, 1_000_000_000);

    await program.methods
      .deposit(new BN(100_000_000))
      .accounts({
        owner: admin.publicKey,
        config: pdas.config,
        userTokenAccount: userToken,
        userShareAccount: userShare,
      })
      .rpc();

    expect(Number((await getAccount(provider.connection, userShare)).amount)).to.eq(100_000_000);
    expect(Number((await getAccount(provider.connection, pdas.tokenVault)).amount)).to.eq(100_000_000);
  });

  it("deposit: после «дохода» доля дорожает", async () => {
    // имитируем доход vault: кидаем 10 токенов напрямую в хранилище
    await mintTo(provider.connection, admin, tokenMint, pdas.tokenVault, admin, 10_000_000);
    await program.methods
      .deposit(new BN(110_000_000))
      .accounts({
        owner: admin.publicKey,
        config: pdas.config,
        userTokenAccount: userToken,
        userShareAccount: userShare,
      })
      .rpc();
    // 110 * 100 / 110 = 100 новых долей
    expect(Number((await getAccount(provider.connection, userShare)).amount)).to.eq(200_000_000);
  });
});
