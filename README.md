# wic - what's in my channel

Just something to peek inside your lightning channel (only for [ldk-node](https://github.com/lightningdevkit/ldk-node)).
Beyond the basic things that your node will already tell you, you can see how does the current commitment tx look like: 

- Anchor outputs
- Pending HTLC outputs (if any)
- Balance going to the local and remote node

```
Current Commitment Transaction: 
Balance to local node: 144710
Balance to remote node: 50001
Transaction fee: 327

Inputs:
  Input 0: OutPoint { txid: 57f5a165516f1ac7d247aa8771b3ad4fd13793455c7271ffa9cc9c1148496e8f, vout: 0 }

Outputs:
  Output 0:
    Value: 0.00000330 BTC sats
    Script Pubkey: OP_0 OP_PUSHBYTES_32 311541beb9213a1212548588bb156d89462ddeaf5c43f8ece551ae218a8c2f08
    Address: bcrt1qxy25r04eyyapyyj5skytk9td39rzmh40t3pl3m892xhzrz5v9uyqd4qcvc

  Output 1:
    Value: 0.00000330 BTC sats
    Script Pubkey: OP_0 OP_PUSHBYTES_32 55f5e8cad2e6da7661c0283b9367fc56c2fa77713bdc5f9a2f195bd3c6b845be
    Address: bcrt1q2h673jkjumd8vcwq9qaexelu2mp05am380w9lx30r9da834cgklqv7fxfq

  Output 2 (HTLC output):
    Value: 0.00004301 BTC sats
    Script Pubkey: OP_0 OP_PUSHBYTES_32 78b385326e8fe15f6f1b8bafdfdb3a8d6f25a596fedc302239af2691994e76e6
    Address: bcrt1q0zec2vnw3ls47mcm3whalke634hjtfvklmwrqg3e4unfrx2wwmnqemlxsj

  Output 3:
    Value: 0.00050001 BTC sats
    Script Pubkey: OP_0 OP_PUSHBYTES_32 36f918f579091d2e07158a629078fea799eafa85e9b7142fbd81bef446b4f915
    Address: bcrt1qxmu33atepywjupc43f3fq78757v74759axm3gtaasxl0g345ly2saa5rj0

  Output 4:
    Value: 0.00144710 BTC sats
    Script Pubkey: OP_0 OP_PUSHBYTES_32 277e6aac02ba6442cbb034c6f2b57eaa3f98e279fd3627c589fa6466778c9f3b
    Address: bcrt1qyalx4tqzhfjy9jasxnr09dt74gle3cnel5mz03vflfjxvauvnuasmnnhcg

```
