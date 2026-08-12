# 研究链路 `--chain-next` 回归与故障注入报告

- 时间: 2026-08-12 10:01:09 +0800
- 工作区: /Users/Shared/chroot/dev/kyuubiki
- 研究: dielectric-screening

## 关键结果
- 基础探索 winner: polyimide_film
- baseline chain round_count: 3
- baseline final_iteration: 4
- baseline convergence_state: no_search_space_progress
- baseline runs: 3
- baseline/input-fingerprint: 4644bd29ec6c100be1e436c9ec12d666cd2d44be778ec79524bc9015b1045cc7
- replay runs: 3
- replay candidate_input_fingerprint: 4644bd29ec6c100be1e436c9ec12d666cd2d44be778ec79524bc9015b1045cc7

## 命令状态（0=成功，1=失败）
| 命令 | 状态 | 说明 |
| --- | --- | --- |
| describe-study | 0 | 校验 study 元信息 |
| initial-explore | 0 | 基础 run 输入 |
| plan-study | 0 | 打印计划 schema |
| chain-next baseline | 0 | round=3 |
| chain-next replay | 0 | 可复现性检查 |
| chain-next rounds=0 | 1 | 预期失败 |
| chain-next bad study | 1 | 预期失败 |
| chain-next missing input | 1 | 预期失败 |

## 故障注入复用快照
rounds=0 output:
```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running `target/debug/kyuubiki-material-explore --chain-next /Users/Shared/chroot/dev/kyuubiki/chain-next-regression/initial.json --rounds 0 --json --out /Users/Shared/chroot/dev/kyuubiki/chain-next-regression/chain-fault-round0.json`
--rounds must be at least 1

```
bad study output:
```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.03s
     Running `target/debug/kyuubiki-material-explore --chain-next /Users/Shared/chroot/dev/kyuubiki/chain-next-regression/initial-bad-study.json --rounds 3 --json --out /Users/Shared/chroot/dev/kyuubiki/chain-next-regression/chain-fault-bad-study.json`
unsupported material study: bogus_study

```
missing input output:
```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.04s
     Running `target/debug/kyuubiki-material-explore --chain-next /tmp/does-not-exist-for-chain-next.json --rounds 3 --json --out /Users/Shared/chroot/dev/kyuubiki/chain-next-regression/chain-fault-missing-input.json`
failed to read /tmp/does-not-exist-for-chain-next.json: No such file or directory (os error 2)

```

## 发现与建议
- chain-next 在固定输入下可重复得到相同 winner 与 candidate_input_fingerprint，适合作为闭环稳定性基线。
- 建议在流水线上加入输入校验（--rounds、material study 一致性、输入文件存在性）以提前拦截无效链路。
