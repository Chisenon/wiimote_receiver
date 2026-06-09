# Wiimote Receiver

WiiリモコンをBluetooth経由でPCに接続し、3Dで可視化するデスクトップアプリケーション。

Wiimoteの加速度センサー・ジャイロセンサー（MotionPlus）のデータをリアルタイムで受信し、3D空間上でリモコンの傾きやボタン入力を視覚的に表示します。

## Demo

https://github.com/user-attachments/assets/5843c6cb-e876-4629-8419-a0c6f9f95f6e

## Features

- **Bluetooth接続**: WiiリモコンをBluetooth経由で自動検出・接続
- **3D可視化**: Three.js によるリアルタイム3D描画でリモコンの傾き・回転を表示
- **ボタン表示**: 押下されたボタンをリアルタイムに表示（A/B/十字キー/+/−/HOME/1/2）
- **MotionPlus対応**: ジャイロセンサー搭載のWiiリモコンPlusに対応、高精度な姿勢推定
- **加速度センサー**: SLERP補間によるスムーズな姿勢フィルタリング
- **キーボードバインド**: WiimoteのボタンをPCのキーボード入力に変換可能
- **マウスルック**: Wiimoteの傾きでマウス操作が可能

## Tech Stack

| Layer       | Technology                   |
| ----------- | ---------------------------- |
| Frontend    | React + TypeScript + Vite    |
| 3D Rendering| Three.js (react-three-fiber) |
| Backend     | Rust (Tauri)                 |
| Input       | wiimote-rs, enigo            |

## Requirements

- Windows / macOS / Linux
- Bluetoothアダプター
- Wiiリモコン（MotionPlus推奨）

## Getting Started

```bash
# 依存関係のインストール
npm install

# 開発サーバー起動
npm run tauri dev

# ビルド
npm run tauri build
```

## How It Works

1. アプリ起動時にWiimoteのスキャンを開始
2. ペアリング済みのWiimoteを自動検出・接続
3. 加速度・ジャイロデータを読み取り、姿勢推定フィルタ（SLERP相補フィルタ）で平滑化
4. 各軸の角度とボタン状態をフロントエンドにイベント送信
5. Three.jsで3Dモデルとしてレンダリング

## Project Structure

```
wiimote_receiver/
├── src/                    # React フロントエンド
│   ├── components/
│   │   ├── WiimoteVisualizer.tsx  # 3D表示コンポーネント
│   │   └── ButtonDisplay.tsx      # ボタン表示コンポーネント
│   └── App.tsx             # メイン画面
├── src-tauri/              # Rust バックエンド
│   └── src/
│       └── lib.rs          # Wiimote受信・姿勢推定処理
└── package.json
```
