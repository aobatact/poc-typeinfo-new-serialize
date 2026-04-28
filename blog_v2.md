<!-- 最終構成案 

タイトル 

derive マクロはもう不要？ Rust nightly の進展で複合型シリアライズの PoC が動いた 

前提メモ（執筆時に反映してください） 

- 対象読者: Rust 中級〜上級（serde / proc macro は前提、vtable・unsafe raw pointer 操作もある程度分かる人） 
- 検証環境: rustc 1.96.0-nightly (362211dc2 2026-03-24) 
- 独自性の軸: 両 feature (type_info + try_as_dyn) を組み合わせた PoC + 実ビルド時間計測を日英問わず公開した記事がまだ見当たらない 
- 読者に残したい状態: 「自分も nightly で触ってみたくなった」 

見出し骨子  -->

# derive マクロはもう不要？ Rust nightly の進展で複合型シリアライズの PoC が動いた

## 要約

[前回](https://zenn.dev/uniquevision/articles/dfc58260217ab6) は type_info を用いて、 proc macro を
用いずにシリアライザを作れないか挑戦しましたが、実装が進んでおらず断念しました。
そこから Rust の対応が進んだので現時点でどこまでできるか試してみたところ、
構造体のシリアライズまではできることを確認できました！

<!-- → 以前書いた記事（リンク）で「複合型は現状無理」と結論づけた直後、 
2026 年初頭にかけて `TypeId::trait_info_of_*` 系の PR が連続マージされ、 
試してみたら struct があっさりシリアライズできた、その驚き。 
→ PoC のコード改変量が意外と少なかった実感も添える。  -->

## この記事で分かること 

- 前回から何が動くようになったか 
- 構造体の `type_info` 経由でのメソッド呼び出し方法
    - `TypeId::trait_info_of_trait_type_id` の使い方
-  reflection 版シリアライザの実ビルド時間（serde+derive との比較） 
<!-- - 対象読者 / 検証環境  -->

## 前回のおさらい（短く） 
前回の記事の時点では、型情報からフィールドの情報はとれても、
そこからそのフィールドがどうやってシリアライズできるかを取得することはできませんでした。
型からどのトレイトの関数を呼ぶ手段がなかったからです。
<!-- → 「TypeId から trait への変換ができない」が最大の壁だったこと、3〜4 行で。 -->

## 何が変わったか（進展ダイジェスト） 
- `TypeId::trait_info_of_trait_type_id` で TypeId → `dyn Ser<S>` の vtable が引けるようになった 
- これにより Struct / Tuple / Array / Reference の reflection が書けるようになった 
-  reflection のみで対応できる型カテゴリが **1 → 5** に拡大 

## 実装: vtable を fat pointer に組み立てる 
→ v2 の肝。`const fn` で `trait_info_of_trait_type_id` から vtable を取り出し、 
field offset と組み合わせて `&dyn Ser<S>` の fat pointer を手で組み立てる。 
→ `SerFieldInfo` / `SerTypeInfo` / `get_reflect_vtable` のコード断片。 
→ なぜこれが必要になったか（前回との対比）。 

## 実装: 2 階層の特殊化（SpecializedSer / SpecializedSerInner） 
→ ユーザー拡張点を残すためユーザー向け（`SpecializedSer`）と 
crate 内向け（`SpecializedSerInner`）を分けた。 
→ ディスパッチ順序: `SpecializedSer` → `SpecializedSerInner` →  reflection フォールバック。
→ `try_as_dyn` での分岐の書き方。 

## 実装: 複合型が動く様子 
→ struct / tuple / 配列 / 参照 の serialize ロジック（最小コード断片）。 
→ Option / Vec / String / HashMap など std 型の特殊化は代表例だけ紹介。 

## ビルド時間を測ってみた 
→ ベンチ設計: 同一の 8 フィールド構造体 200 個、serde+derive 版 と PoC  reflection 版で比較。 
→ 結果（5 回中央値）: 

| mode | serde+derive | type_info  reflection  | 比 | 
|---------|-------------:|---------------:|-------:| 
| debug | 0.567 s | 0.425 s | 0.75x | 
| release | 5.295 s | 12.586 s | 2.38x | 

→ 解釈: 
- debug（日常イテレーション）は reflection のほうが既に **速い**（proc macro 展開が重い）
- release は現状 **遅い**（ブランケット impl の単相化 + LLVM 最適化が膨らむ） 
→ ベンチのコードはリポジトリの `bench/` に公開。 

## 残っている課題 
- enum の reflection 対応（type_info 側の API 整備待ち） 
- slice等の UnSized な型 を type_info 経由で処理できない（try_as_dyn の `?Sized` 対応待ち） 
- `MAX_FIELDS = 20` の暫定上限（`const` context の配列長制約）
- release 時のコンパイル時間が serde より重い 

## 設計判断のメモ 
- なぜ 2 階層の特殊化を入れたか（ユーザー拡張点を残すため） 
- なぜ vtable を手で組み立てる方針にしたか（他に選択肢がなかった） 
- MAX_FIELDS=20 の妥協理由

## まとめ 
- 前回「無理」と結論づけた複合型が、実際に動くようになった 
- proc macro を使わない serde 互換の設計は、もう紙の上の話ではない 
- dev ビルドは既に速い、release は今後の課題
- 興味が湧いたら PoC リポジトリを clone して nightly で触ってみてほしい 

---
**参考リンク** 
- 前回の記事（v1） 
- reflection-and-comptime プロジェクトゴール
- `TypeId::trait_info_of` PR (#152003)
- Reflection MVP PR (#146923) 
- `try_as_dyn` PR (#150033)
- PoC リポジトリ 

---
ここから先は 

本文はご自身の言葉で書いてください。 骨子をそのままコピペせず、以下を意識すると差別化が残ります:

- 冒頭: 「前回無理と書いた直後に動いた」という体感温度を1〜2文で 
- 実装パート: 公式ドキュメントに書いていない「書いてみて分かった」判断を入れる（2階層特殊化、MAX_FIELDS、vtable 手組み） 
- ベンチ: 非対称な結果に対するあなた自身の解釈を正直に書く（「期待と違った」で OK） 
- まとめ: 「触ってみてほしい」で締める（Q3-3 で決めた着地点） 