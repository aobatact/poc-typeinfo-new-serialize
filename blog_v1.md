# derive マクロはもう不要？ nightly の type_info でシリアライザを書いてみた

## はじめに

Rust のビルドが重いと感じたことはありませんか？
特に `serde` などの derive マクロを多用するプロジェクトでは、proc macro の展開処理がビルド時間の大きな割合を占めることがあります。

この問題を解決するかもしれない機能が、現在 Rust nightly で開発が進んでいます。
それが、 `feature(type_info)` と `feature(try_as_dyn)` です。本記事では、これらの機能を使って derive マクロなしでシリアライズを実装してみたので、
それをもとに利用法と現状などを紹介していきたいです。

## この記事で分かること

- `type_info` / `try_as_dyn` で何ができるか
- derive マクロを使わないシリアライズの実装方法
- 現状の制約と将来の展望

**対象読者**: Rust 中級者〜上級者（serde や proc macro を理解している人）

**検証環境**: Rust 1.95.0-nightly (85eff7c80 2026-01-15)

## type_info とは？

Rust は、静的型付けな言語です。
全てコンパイル時に処理されるのでランタイム時には型消去され、型のメタ情報を取得することは言語レベルではできないです。
そのためリフレクションなどをしたい場合、proc macro などを駆使したりして、ユーザーが型情報をコードから取得して、独自にハンドリングしないといけないです。
Rust はマクロが強力なため、型情報をパースして処理することでコードを生成することが多いです。代表的なものでいえば、[serde](https://github.com/serde-rs/serde)というシリアライザのライブラリでしょう。
[bevy](https://bevy.org/) という Rust製のゲームエンジンでも使われています。
中にも [bevy_reflect](https://docs.rs/bevy_reflect/latest/bevy_reflect/) というリフレクション用のcrate があったりします。

ただそのようなリフレクションのcrateには制約があります。
まず proc macro はそのcrateのコードに対してしか適用できないです。
依存元の crateにある型に対してリフレクションをしたい場合、ダミーでもう一度型定義を記述するようなことが必要になってきます。

また orphan 則とかも厄介だったりします。
ある依存元 crate Aの構造体に、crate Bのtrait をimplすることはできない仕様になっています。
これによって B側が Aの構造体をfeatureで持っていたり、その逆ではないケースでは、ラッパー型を用意したりとかなりの手間です。
Rustのシリアライザのライブラリのデファクトがserdeでほぼ統一されているのは、新規のシリアライザcrateだと他のcrateに対応してもらうか、自分で対応しないかぎり、重ねて使うのは難しく、直交的にcrateが利用されないです。

ここに対して、コンパイル時のリフレクションを強化するゴールが追加されました。

[reflection-and-comptime](https://rust-lang.github.io/rust-project-goals/2025h2/reflection-and-comptime.html)

この中に `feature(type_info)`があります。
これは`const fn`で、型情報を取得できるAPIを提供するものです。
これによってproc macroでパースしなくとも、型情報を取得できます。
コード生成が出来るわけではないので、proc macro よりもできることは少ないですが、
コンパイル時に解決されるので、ほぼ0コストで型に応じた処理の分離ができます。
またコンパイラが型情報を挿入してくれるため、自分のcrateの型でなくともアクセスできます。(※ただし、privateのものは制限される予定)

||proc macro|type_info|
|-|-|-|
|コード生成|〇|×|
|コンパイル時解決|〇|〇|
|自分のcrate外の型情報|x|〇|
|コンパイル速度|x|多分〇|

## feature の説明

### type_info

`type_info` は、型の情報をコンパイル時に取得できる機能です。

```rust
#![feature(type_info)]
use std::mem::type_info::Type;

let type_val = const { Type::of::<i32>() };
```
このようにconstで型の情報を取得できます。
ジェネリクスでもいいので、何が入ってくるかわからないジェネリクスの型に対して、
型情報をもとに分岐処理ができるようになります。
型の種類（プリミティブ、タプル、配列など）を判別できる
`const` で取得するため、実行時のコストを抑えられます。
現在進行形で対応が進んでいて、[構造体の情報取得はまだ nightlyにさえmergeされてないです。](https://github.com/rust-lang/rust/pull/151142) (2026/1/30現在)　
ですが、ここ数か月で実装されてきているので、期待できる featureです。

### try_as_dyn

さて、もう一つ feature を紹介したいです。
それは `try_as_dyn`です。
`try_as_dyn` は、ある型が特定の trait を実装している場合に、動的ディスパッチに変換できる機能です。

```rust
#![feature(try_as_dyn)]

if let Some(specialized) = std::any::try_as_dyn::<_, dyn MyTrait>(value) {
    specialized.do_something();
}
```
実行時に trait の実装有無を確認し、条件分岐することができます。
trait の特殊化（specialization）よりも実現性が高いです。
zulip では trait の特殊化を消してこちらにするという話も出ていました。
最適化によって分岐が消えることも期待できます。

## これでシリアライズできると何がうれしいか

### 1. proc macro が不要になる → ビルド速度の高速化

従来の serde では、各構造体に `#[derive(Serialize)]` を付ける必要がありました。

```rust
// 従来の方法
#[derive(Serialize)]
struct User {
    name: String,
    age: u32,
}
```

proc macro は各型ごとにコードを生成するため、型が増えるほどビルド時間が増加します。`type_info` を使えば、実行時に型情報を取得して処理できるため、この問題を回避できます。

### 2. 任意の型をシリアライズできる

`type_info` を使えば、serde を依存に持たないライブラリの型でも、汎用的にシリアライズできます。
これによって、シリアライザはserdeでないと対応が膨大という問題を回避できます。
エコシステムの多様化が促進されていくことが期待できます。

### 3. シリアライズのカスタマイズができる

`try_as_dyn` も併用することで、汎用的な型処理とは別に特殊なシリアライズ方法を実装できます。
同じcrate内なら、型の条件分岐の処理に追加していけばいいですが、シリアライザをライブラリとして利用する側のcrateからはロジックのカスタマイズがそのままではできないので、
`try_as_dyn`を利用することで独自型に独自ロジックを埋め込めるようになります。

また現状のserdeだと型とシリアライザを対応させてカスタマイズができませんが、それもできるようになるはずです。

## 実装例（PoC）

実際に `type_info` と `try_as_dyn` を使ってシリアライズを実装してみます。
全体は [PoC](https://play.rust-lang.org/?version=nightly&mode=debug&edition=2024&gist=de9a8568b6c9ad2ff8457746b4340851) に置いておきます。

### 基本構造

まず、シリアライズのための trait を定義します。

```rust
#![feature(try_as_dyn)]
#![feature(type_info)]
use std::mem::type_info::Type;

pub trait Ser {
    fn serialize<S: Serializer + 'static>(&self, serializer: &mut S);
}

pub trait Serializer {
    fn serialize_str(&mut self, value: &str);
    fn serialize_i32(&mut self, value: i32);
    fn serialize_u32(&mut self, value: u32);
    fn serialize_bool(&mut self, value: bool);
    fn serialize_f64(&mut self, value: f64);
    // ... 他のプリミティブ型
}
```

### try_as_dyn を用いた特殊化

特定の型に対してカスタムのシリアライズを提供したい場合、`try_as_dyn` を使って特殊化できます。

```rust
pub trait SpecializedSer<S: Serializer> {
    fn specialized_serialize(&self, serializer: &mut S);
}

// Option<T> に対する特殊化の例
impl<T: Ser> SpecializedSer<JsonSerializer> for Option<T> {
    fn specialized_serialize(&self, serializer: &mut JsonSerializer) {
        match self {
            Some(value) => serializer.serialize_some(value),
            None => serializer.serialize_none(),
        }
    }
}
```



### type_info を用いた型分岐

`type_info` を使って、型の種類に応じて処理を分岐します。

```rust
impl<T: 'static> Ser for T {
    fn serialize<S: Serializer + 'static>(&self, serializer: &mut S) {
        // まず特殊化された実装があるか確認
        if let Some(specialized) = std::any::try_as_dyn::<_, dyn SpecializedSer<S>>(self) {
            specialized.specialized_serialize(serializer);
            return;
        }

        // type_info を使って型に応じた処理
        let type_val = const { Type::of::<T>() };
        match type_val.kind {
            std::mem::type_info::TypeKind::Bool(_) => {
                unsafe {
                    let b = *(self as *const T as *const bool);
                    serializer.serialize_bool(b);
                }
            },
            std::mem::type_info::TypeKind::Int(int) => {
                if int.signed {
                    unsafe { match int.bits {
                        32 => serializer.serialize_i32(*(self as *const T as *const i32)),
                        // ... 他のサイズ
                        _ => unreachable!(),
                    }}
                } else {
                    unsafe { match int.bits {
                        32 => serializer.serialize_u32(*(self as *const T as *const u32)),
                        // ... 他のサイズ
                        _ => unreachable!(),
                    }}
                }
            },
            std::mem::type_info::TypeKind::Float(float) => {
                unsafe {
                    match float.bits {
                        64 => serializer.serialize_f64(*(self as *const T as *const f64)),
                        _ => unreachable!(),
                    }
                }
            },
            // ... 他の型
            _ => todo!(),
        }
    }
}
```

### 使用例

```rust
fn main() {
    let mut json = JsonSerializer {
        output: String::new()
    };
    (42_u32).serialize(&mut json);
    println!("{}", json.output); // => 42
}
```

## 現状の問題点

この PoC を実装する中で、いくつかの課題が見つかりました。

### 1. TypeId から trait への変換ができない

`type_info` で型の構造（タプルのフィールド、配列の要素など）は取得できますが、それらの要素に対して `Ser` trait のメソッドを呼び出す方法がありません。

```rust
std::mem::type_info::TypeKind::Tuple(tuple) => {
    tuple.fields.iter().for_each(|field| {
        unsafe {
            let field_ptr = (self as *const T as *const u8).add(field.offset);
            // 問題: field の型に対して serialize を呼び出せない
            // TypeId は分かるが、それを Ser trait に変換する方法がない
        }
    });
},
```

### 2. try_as_dyn はジェネリクスの型 T が分かっていないといけない

`try_as_dyn` は型パラメータが静的に決まっている必要があるため、動的に取得した型情報に対しては使用できません。

### 3. 複合型の対応が困難

上記の制約により、タプル、構造体、配列などの複合型をシリアライズすることが現状では困難です。

## 将来の展望

[Reflection TypeId::trait_info_of](https://github.com/rust-lang/rust/pull/152003) などの PR で議論が進んでおり、将来的にはこれらの問題が解決される可能性があります。

また別軸で、proc macro ではなく [declarative macro でderive macro を実装できるようにする](https://rust-lang.github.io/rfcs/3698-declarative-derive-macros.html)
という RFCもあり、こちらは orphan 則周りは解決しないですが、コンパイル時間の短縮は期待できそうです。

## まとめ

- `feature(type_info)` と `feature(try_as_dyn)` を使えば、derive マクロなしでシリアライズの基盤を実装できる
- プリミティブ型については問題なく動作する
- 複合型（タプル、構造体など）は TypeId から trait への変換ができないため、現状では対応が難しい
- Rust の今後の開発により、これらの制約が解消される可能性がある

proc macro に依存しないシリアライズが実現すれば、ビルド時間の大幅な短縮が期待できます。Rust の型システムの進化に引き続き注目していきたいと思います。

---

**参考リンク**

- [reflection and comptime](https://rust-lang.github.io/rust-project-goals/2025h2/reflection-and-comptime.html)
- [Reflection MVP](https://github.com/rust-lang/rust/pull/146923)
- [Add try_as_dyn and try_as_dyn_mut](https://github.com/rust-lang/rust/pull/150033)
- [PoC](https://play.rust-lang.org/?version=nightly&mode=debug&edition=2024&gist=de9a8568b6c9ad2ff8457746b4340851)
