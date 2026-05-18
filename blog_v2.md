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
- reflection 版シリアライザの実ビルド時間（serde+derive との比較） 
<!-- - 対象読者 / 検証環境  -->

## 前回のおさらい（短く） 
前回の記事の時点では、型情報からフィールドの型はとれても、その型をどうシリアライズするかを取り出す手段がありませんでした。
型ごとにどのトレイトが実装されているかは当時でも`try_as_dyn`で確認できましたが、これは型`T`が分かっている前提のAPIで、`TypeId`からトレイトオブジェクトを取得することはできなかったためです。
そのため、配列の要素や構造体のフィールドのように`TypeId`としてしか得られない型を処理できませんでした。

<!-- → 「TypeId から trait への変換ができない」が最大の壁だったこと、3〜4 行で。 -->

## std側で何が変わったか

`TypeId::trait_info_of_trait_type_id` で `TypeId` から `dyn Ser<S>` の vtable が引けるようになりました！
これにより、構造体フィールドや配列要素の `TypeId` から vtable を取り出し、再帰的に処理することができます。

また構造体などの型のサポートも広がり、フィールドのoffsetと`TypeId`からそのトレイトオブジェクトを取得できるようになりました。
これにより Struct / Tuple / Array / Reference の reflection が書けるようになり、
プリミティブだけだった前回から対応できる型の範囲は大幅に広がりました。

try_as_dynが `T: Sized`で、slice等の Unsized な型 を type_info 経由で処理できないという課題もありましたが、これは自分がPRを書いて解決させました！
[rust-lang/rust#156104](https://github.com/rust-lang/rust/pull/156104)


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

型の構造情報だけでは表現しきれないシリアライズ処理は、別のトレイトを用意してそちらで実装することで特殊化できます。
`#[feature(min_specialization)]` のような既存の特殊化と違って、この方式はトレイト間の継承関係を持たず、対象のトレイトが実装されていればそちらを優先するだけのシンプルな仕組みです。既存の特殊化はトレイト同士の関係から unsound になり得ますが、この方式の方が安定化に近いとみられています。

`serde`と違い、シリアライザは関数の型パラメーターではなく、トレイト自体の型パラメーターにしてあります。これによって、シリアライザと対象の型のペアで特殊化できます。
例えば、時刻型を独自に扱いたいシリアライザ向けに、汎用のシリアライズ経路の外でその型の挙動を差し替える、といったことが可能です。現状の`serde`だと`deserialize_with`を使って構造体のあるフィールドに対して処理の特殊化ができますが、型単位ではできません。
また、すこし工夫した点として、特殊化に使うトレイトを内部用と外部用とで分けることで、stdの型でもシリアライザに合わせた特殊化を差し込む余地を用意しています。

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

せっかくなのでビルド時間がどうなったかを確認してみました。

→ ベンチ設計: 同一の 8 フィールド構造体 200 個、serde+derive 版 と PoC reflection 版で比較。 
→ 結果（5 回中央値）: 

| mode | serde+derive | type_info reflection | 比 | 
|---------|-------------:|---------------:|-------:| 
| debug | 0.567 s | 0.425 s | 0.75x | 
| release | 5.295 s | 12.586 s | 2.38x | 

### 解釈
debugは reflection のほうが速いのは、`serde` は proc macro 展開が重いからだと思われます。
ただ、 release で現状遅いのはブランケット impl の単相化 と LLVM 最適化が膨らむのが原因かもしれません。

→ ベンチのコードはリポジトリの `bench/` に公開。 

## 残っている課題 
一番大きいのは **enum の reflection 対応** で、これは type_info 側の API がまだ整備されていないため、[rust-lang/rust#156403](https://github.com/rust-lang/rust/pull/156403) の進展を待つ必要があります。

実装面では **`MAX_FIELDS = 20` の暫定上限** が残っています。これは `const` 文脈で動的長の配列を扱えないことから来ている制約で、`const heap` のような機能が入るまでは外せません。

性能面では前述の通り **release 時のコンパイル時間が serde より重い** という問題があり、ブランケット impl の単相化や LLVM 最適化の影響と思われます。ここはまだ調査の余地があります。

設計上の制約として **`'static` 制約** も付いて回ります。`TypeId::of` や `try_as_dyn`、`DynMetadata<dyn Ser<S>>` のいずれも暗黙の `'static` を要求するため、現状では `&'static T` しか扱えません。最近の PR で `Type::of` 側からは `'static` 境界が外れたものの、`try_as_dyn` ベースの特殊化経路に非 `'static` の `TypeId` を持ち込むと soundness を壊しうるため、reflection 経路だけ分離して外すといった工夫が必要になりそうです。

最後に、serde で言うところの **属性対応**（フィールド名のリネームなど）もまだありません。現状、`type_info`では属性の情報はとれないのでここも std側の対応を待つことになります。

## まとめ 
前回は不可能だった複合型のシリアライズが、実際に動くところまで持っていけました！
serdeでは難しかった型単位の拡張性も、特殊化の仕組みで担保しています。
dev ビルドは serde より速いものの、release は遅い状態で、enum など未対応の型もあるので、ここから先は std 側の進展を待つ形になります。
興味が湧いたら PoC リポジトリを clone して nightly で触ってみてください。

---
**参考リンク** 
- 前回の記事（v1） 
- reflection-and-comptime プロジェクトゴール
- `TypeId::trait_info_of` PR (#152003)
- Reflection MVP PR (#146923) 
- `try_as_dyn` PR (#150033)
- PoC リポジトリ 
