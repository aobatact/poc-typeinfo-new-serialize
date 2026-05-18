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
型ごとにどうトレイトが実装されているかは当時でも`try_as_dyn`で取得できましたが、
`T`の型がわかっている前提で、`TypeId`からはトレイトオブジェクトを取得することができないままでした。なので配列の要素や構造体のフィールドの型として取得したの`TypeId`を処理することができなかったのです。

<!-- → 「TypeId から trait への変換ができない」が最大の壁だったこと、3〜4 行で。 -->

## std側で何が変わったか

`TypeId::trait_info_of_trait_type_id` で TypeId → `dyn Ser<S>` の vtable が引けるようになりました！
これによってそれぞれの内側の型の`TypeId`から、vtableを取得して、再帰的に処理することができました。
また構造体などの型のサポートも広がり、フィールドのoffsetと`TypeId`からそのトレイトオブジェクトを取得できるようになりました。
これにより Struct / Tuple / Array / Reference の reflection が書けるようになり、
プリミティブだけだった前回から対応できる型の範囲は大幅に広がりました。

## 実装: vtable を fat pointer に組み立てる 

上記の関数を活用するために以下のように処理をします。
まず `const fn` で `trait_info_of_trait_type_id` から vtable を取り出し、 
field offset と組み合わせて `&dyn Ser<S>` の fat pointer を手で組み立てます。
ここで fat pointer 取得とその構造化をコンパイル時に先に処理してしまうことによって、
実行時に 型ごとの分岐がいらなくなるようにしています。

vtable 取得は `TypeId` だけを入力にとる `const fn` として書けます:

```rust
const fn get_reflect_vtable<S: Serializer + 'static>(type_id: TypeId) -> DynMetadata<dyn Ser<S>> {
    let trait_id = TypeId::of::<dyn Ser<S>>();
    match type_id.trait_info_of_trait_type_id(trait_id) {
        Some(t) => unsafe { std::mem::transmute(t.get_vtable()) },
        None => panic!("type does not implement Ser"),
    }
}
```

そしてフィールドごとに「offset + vtable」を `SerFieldInfo` に詰めておけば、
実行時には親オブジェクトの先頭ポインタにオフセットを足して fat pointer を組むだけで `&dyn Ser<S>` が手に入ります:

```rust
struct SerFieldInfo<S: 'static> {
    name: &'static str,
    offset: usize,
    vtable: DynMetadata<dyn Ser<S>>,
}

impl<S: 'static> SerFieldInfo<S> {
    const unsafe fn to_dyn<T: ?Sized>(&self, ptr: &T) -> &dyn Ser<S> {
        unsafe {
            let field_ptr = (ptr as *const T as *const u8).add(self.offset);
            let fat_ptr = std::ptr::from_raw_parts::<dyn Ser<S>>(field_ptr as *const (), self.vtable);
            &*fat_ptr
        }
    }
}
```

これを型ごとに 1 度だけ走らせて、`MaybeUninit` の固定長配列に詰めたものを `const { TypeSer::<S>::of::<T>() }` で取り出します。`TypeSer` を const 評価の段階で完成させてしまうことで、実行時は固定長配列を舐めるだけで済みます。

```rust
match type_info.kind {
    TypeKind::Struct(struct_fields) => {
        let mut array = [const { MaybeUninit::<SerFieldInfo<S>>::uninit() }; MAX_FIELDS];
        let mut i = 0;
        while i < struct_fields.fields.len() && i < MAX_FIELDS {
            let field = &struct_fields.fields[i];
            array[i] = MaybeUninit::new(SerFieldInfo {
                name: field.name,
                offset: field.offset,
                vtable: get_reflect_vtable::<S>(field.ty),
            });
            i += 1;
        }
        TypeSer::Struct { fields: array, len: i }
    }
    // ... Tuple / Array / Reference / Primitive も同じノリ
}
```

ただここにはトレードオフがあって、これによって対応できるフィールドの数が固定化されてしまいます (PoC では `MAX_FIELDS = 20`)。`const heap`等が実装されるまでこの制限はとれません。

## 実装: 2 階層の特殊化（SpecializedSer / SpecializedSerInner） 

型の構造の情報からでは、表現できないシリアライズの表現を、別のトレイトで実装することで特殊化をすることができてます。
今までの型の特殊化 `#[feature(min_specialization)]` などと違って、この特殊化はトレイト間の継承関係はなく、単にそのトレイトが実装されているならそのトレイトで乗っ取るということをしています。トレイト同士の関係でunsoundになる既存の特殊化と違ってこちらの方が安定化が近いとみられているます。

`serde`と違い、シリアライザは関数の型パラメーターではなく、トレイト自体の型パラメーターにしてあります。これによって、シリアライザと対象の型のペアで特殊化できます。
例えば、時刻用の定義があるシリアライザ向けに汎用のシリアライズの仕組外の方法でシリアライザを実装させることができます。現状の`serde`だと`deserialize_with`を使って構造体のあるフィールドに対して処理の特殊化ができますが、型単位ではできません。
また、すこし工夫した点として、特殊化に使うトレイトを内部用と外部用とで分けることで、
stdの型でもそのシリアライザにあった特殊化ができる余地を用意しています。

トレイトの定義はこれだけです:

```rust
pub trait Ser<S: Serializer> {
    fn serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error>;
}

// ユーザー側で使う特殊化
pub trait SpecializedSer<S: Serializer> {
    fn specialized_serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error>;
}

// std向けに使う特殊化 (crate 内部用)
pub(crate) trait SpecializedSerInner<S: Serializer> {
    fn specialized_serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error>;
}
```

すべての `T: 'static` に対して blanket impl を一発で書き、その中で `try_as_dyn` を使って優先度順に特殊化を試し、最後に reflection に落ちます:

```rust
impl<T: 'static, S: Serializer + 'static> Ser<S> for T {
    fn serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error> {
        // 1. ユーザー定義の特殊化があれば最優先
        if let Some(specialized) = std::any::try_as_dyn::<_, dyn SpecializedSer<S>>(self) {
            specialized.specialized_serialize(serializer)
        // 2. std型用の crate 内部特殊化
        } else if let Some(specialized) = std::any::try_as_dyn::<_, dyn SpecializedSerInner<S>>(self) {
            specialized.specialized_serialize(serializer)
        // 3. それ以外は reflection でフィールドを舐める
        } else {
            let type_ser = const { TypeSer::<S>::of::<T>() };
            // ... TypeSer の variant ごとに分岐
        }
    }
}
```

`SpecializedSer` の枠を 1 つユーザーに残しつつ、`String` や `Vec<T>` などは `SpecializedSerInner` 側で先に実装してあるので、std 型の挙動を奪われずに済みます。

## 実装: 複合型が動く様子 

reflection 経路では、`const` で組み立てた `TypeSer<S>` を `match` するだけで struct / tuple / 配列 / 参照 が動きます。たとえば struct の枝はこうなります:

```rust
TypeSer::Struct { fields, len } => unsafe {
    let fields = fields[..len].assume_init_ref();
    let mut s = serializer.serialize_struct(std::any::type_name::<T>(), len)?;
    for field in fields {
        // SerFieldInfo::to_dyn が offset + vtable で &dyn Ser<S> を組み立てる
        let field_value = field.to_dyn(self);
        s.serialize_field(field.name, field_value)?;
    }
    s.end()
},
TypeSer::Array { len, elem } => unsafe {
    let mut seq = serializer.serialize_seq(Some(len))?;
    for i in 0..len {
        let field_ptr = (self as *const T as *const u8).add(i * elem.size);
        let field_value = elem.to_dyn(&*field_ptr.cast::<()>());
        seq.serialize_element(field_value)?;
    }
    seq.end()
},
TypeSer::Reference { referent } => unsafe {
    let pointee_ptr = *(self as *const T as *const *const u8);
    let pointee = referent.to_dyn(&*pointee_ptr.cast::<()>());
    pointee.serialize(serializer)
},
```

std 型は `SpecializedSerInner` 側で短く書きます。たとえば `Option<T>` と `Vec<T>` はそれぞれこれだけ:

```rust
impl<T: Ser<S>, S: Serializer> SpecializedSerInner<S> for Option<T> {
    fn specialized_serialize(&self, serializer: &mut S) -> Result<S::Ok, S::Error> {
        match self {
            Some(value) => serializer.serialize_some(value),
            None => serializer.serialize_none(),
        }
    }
}

// Vec<T> や Box<T>, Arc<T> ... は Deref に委譲するマクロでまとめて生やしている
specialized_ser_via_deref_inner!(std::vec::Vec<T>, T);
specialized_ser_via_deref_inner!(std::boxed::Box<T>, T: ?Sized);
```

実際に使う側からは proc macro なしでこう書けます:

```rust
struct Point { x: f64, y: f64 }

let mut json = JsonSerializer::new_vec();
(Point { x: 1.0, y: 2.0 }).serialize(&mut json).unwrap();
assert_eq!(json.as_str(), r#"{"x":1,"y":2}"#);
```


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



<!-- ● PR の内容を確認しました。 

  PR 152381 が変えること
  - intrinsics::type_id と Type::of（リフレクションAPI）から 'static 制約が外れた
  - ただし stable な TypeId::of には 'static 境界が残ったまま
  - 非 'static の TypeId を得たい場合は Type::of::<(T,)>().kind で取り出す回避策が想定されている

  このリポジトリで外せるかどうか

  結論：「リフレクション経路だけなら理屈上は外せるが、現在の設計全体としては難しい」 です。

  理由を箇所別に：

  1. TypeSer::<S>::of::<T>()（lib.rs:582）
  現状 TypeId::of::<T>() を使っているので 'static が必須。PR 152381 のおかげで
  Type::of::<T>()(type_info feature) に切り替えれば T: 'static は不要にできる。
  2. get_reflect_vtable（lib.rs:472）
  内部で TypeId::of::<dyn Ser<S>>() を呼んでいる。dyn Ser<S> 自体に暗黙の 'static があるので、ここは
  S: 'static がそのまま要る。
  3. ブランケット impl の T: 'static（lib.rs:587）
  try_as_dyn::<_, dyn SpecializedSer<S>>(self) が値の TypeId を必要とするので、これも基本 'static
  前提。type_id intrinsic 直叩きに置き換えれば回避は可能だけど、try_as_dyn 側の実装次第。
  4. DynMetadata<dyn Ser<S>>
  トレイトオブジェクトの暗黙 'static。dyn Ser<S> + 'a
  化するとシグネチャ全体が高階寿命だらけになって割に合わない。

  実用的な提案
  - PR 本文も明記している通り、非 'static 型の TypeId を使うと
  他のクレートが保持している不変条件（同一 TypeId ⇒ 同一型）を壊しうるので、try_as_dyn ベースの
  specialization 経路に持ち込むのは soundness 的に危険。
  - 効果が大きいのはたぶん &'a T のようなライフタイム付き参照を直接 serialize
  出来るようになること、ぐらい。今は &'static T しか入らない。
  - やるなら「リフレクション専用の Ser 派生 trait（SpecializedSer
  不使用）」を別経路で用意して、そっちだけ 'static を外す、という分離が現実的だと思います。 -->