import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { assert } from "chai";
import { Helloworld } from "../target/types/helloworld";

describe("helloworld", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.Helloworld as Program<Helloworld>;
  type Event = anchor.IdlEvents<typeof program["idl"]>;
  const getEvent = async <E extends keyof Event>(
    eventName: E,
    methodName: keyof typeof program["methods"]
  ) => {
    let listenerId: number;
    const event = await new Promise<Event[E]>((res) => {
      listenerId = program.addEventListener(eventName, (event) => {
        res(event);
      });
      console.log("calling rpc method", methodName);
      return program.methods[methodName]().rpc();
    });
    await program.removeEventListener(listenerId);

    return event;
  };

  it("create and emit event", async () => {
    const event = await getEvent("countChangeEvent", "create");
    assert.strictEqual(event.data.toNumber(), 0);
    assert.strictEqual(event.label, "create");
  });

  it("increment counter and emit event", async () => {
    const event = await getEvent("countChangeEvent", "increment");
    assert.strictEqual(event.data.toNumber(), 0);
    assert.strictEqual(event.label, "inc");
  });
});
