import * as web3 from "@solana/web3.js";
// Manually initialize variables that are automatically defined in Playground
const PROGRAM_ID = new web3.PublicKey("B7BQZ17FQTBfXA1UpsmJVLZzzHVzddEfy9ZQZ5CnfFjz");
const connection = new web3.Connection("http://localhost:8899", "confirmed");
const wallet = { keypair: web3.Keypair.generate() };

// Client
async function main() {
    console.log("My address:", wallet.keypair.publicKey.toString());
    const balance = await connection.getBalance(wallet.keypair.publicKey);
    console.log(`My balance: ${balance / web3.LAMPORTS_PER_SOL} SOL`);
    const signature = await connection.requestAirdrop(wallet.keypair.publicKey, 100 * web3.LAMPORTS_PER_SOL)
    await connection.confirmTransaction(signature);
}