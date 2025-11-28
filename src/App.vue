<script setup lang="ts">
import { ref, onMounted } from "vue";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { GoogleGenerativeAI } from "@google/generative-ai";
import mascotImg from "./assets/mascot.png";
import charImg from "./assets/char.png";
import dialogueImg from "./assets/dialogue.png";

const API_KEY = "AIzaSyCsMdS9v6totSlUtTlPZ4BR9v0BFrgLbLk";
const genAI = new GoogleGenerativeAI(API_KEY);

const options = ref<{ tsundere: string; sweet: string; funny: string } | null>(null);
const loading = ref(false);
const error = ref("");
const showOverlay = ref(false);
const currentEventPayload = ref<string | null>(null);

// Start analysis when the floating widget is clicked
const startAnalysis = async () => {
  loading.value = true;
  error.value = "";
  options.value = null;
  currentEventPayload.value = null;
  await invoke("analyze");
};

// Close overlay and return to widget mode
const closeOverlay = async () => {
  showOverlay.value = false;
  options.value = null;
  error.value = "";
  await invoke("reset_window");
};

const analyzeImage = async (base64Image: string) => {
  loading.value = true;
  error.value = "";
  
  try {
    const model = genAI.getGenerativeModel({ model: "gemini-1.5-flash" });
    const prompt = `Analyze the chat history in this image. The last message is the one to reply to. 
    Provide 3 distinct reply options in a Galgame style (in Chinese):
    1. Tsundere (傲娇 - Cold but caring)
    2. Sweet/Gentle (温柔 - Supportive and kind)
    3. Funny/Meme (搞笑 - Humorous or sarcastic)
    
    Output ONLY a valid JSON object with keys: 'tsundere', 'sweet', 'funny'. Do not include markdown formatting.`;

    const result = await model.generateContent([
      prompt,
      {
        inlineData: {
          data: base64Image,
          mimeType: "image/png",
        },
      },
    ]);

    const response = await result.response;
    const text = response.text();
    const jsonStr = text.replace(/```json/g, "").replace(/```/g, "").trim();
    options.value = JSON.parse(jsonStr);
  } catch (e: any) {
    console.error("Analysis Error:", e);
    error.value = "Error: " + (e.message || JSON.stringify(e));
  } finally {
    loading.value = false;
  }
};

onMounted(async () => {
  await listen<string>("analyze-chat", async (event) => {
    showOverlay.value = true;
    currentEventPayload.value = event.payload;
    await analyzeImage(event.payload);
  });

  await listen("analyze-error", (event) => {
    console.error(event.payload);
    error.value = "无法捕获窗口: " + JSON.stringify(event.payload);
    showOverlay.value = true;
  });
});

const copyToClipboard = async (text: string) => {
  try {
    await navigator.clipboard.writeText(text);
    // Visual feedback could be added here
    closeOverlay();
  } catch (err) {
    console.error("Failed to copy: ", err);
  }
};

const retryAnalysis = () => {
  if (currentEventPayload.value) {
    analyzeImage(currentEventPayload.value);
  } else {
    closeOverlay();
    startAnalysis();
  }
};
</script>

<template>
  <!-- Floating Widget Mode -->
  <div v-if="!showOverlay" class="floating-widget" @click="startAnalysis" data-tauri-drag-region>
    <div class="mascot-container">
      <img :src="mascotImg" class="mascot-icon" alt="AI Assistant" />
      <div class="pulse-ring"></div>
    </div>
  </div>

  <!-- Fullscreen Overlay Mode -->
  <div v-else class="galgame-overlay">
    <!-- Character Art -->
    <img :src="charImg" class="character-art" alt="Character" />

    <!-- Main Dialogue Area -->
    <div class="dialogue-container">
      <img :src="dialogueImg" class="dialogue-bg" alt="UI" />
      
      <div class="dialogue-content">
        <div class="header">
          <span class="name-tag">AI Assistant</span>
          <button class="close-btn" @click="closeOverlay">×</button>
        </div>

        <div class="message-area">
          <div v-if="loading" class="typing-effect">
            正在观察你的对话...<span class="cursor">|</span>
          </div>

          <div v-else-if="error" class="error-msg">
            {{ error }}
            <button @click="retryAnalysis" class="retry-btn">重试</button>
          </div>

          <div v-else class="instruction-text">
            请选择一个回复选项...
          </div>
        </div>
      </div>
    </div>

    <!-- Options Selection (Floating above dialogue) -->
    <div v-if="options" class="options-wrapper">
      <div class="option-btn tsundere" @click="copyToClipboard(options.tsundere)">
        <span class="tag">傲娇</span>
        <span class="text">{{ options.tsundere }}</span>
      </div>
      <div class="option-btn sweet" @click="copyToClipboard(options.sweet)">
        <span class="tag">温柔</span>
        <span class="text">{{ options.sweet }}</span>
      </div>
      <div class="option-btn funny" @click="copyToClipboard(options.funny)">
        <span class="tag">搞笑</span>
        <span class="text">{{ options.funny }}</span>
      </div>
    </div>
  </div>
</template>

<style>
@import url('https://fonts.googleapis.com/css2?family=Noto+Sans+SC:wght@400;700&display=swap');

html, body, #app {
  margin: 0;
  padding: 0;
  width: 100%;
  height: 100%;
  background: transparent;
  overflow: hidden;
  user-select: none;
  font-family: 'Noto Sans SC', sans-serif;
}
</style>

<style scoped>
/* Floating Widget */
.floating-widget {
  width: 100vw;
  height: 100vh;
  display: flex;
  justify-content: center;
  align-items: center;
  cursor: pointer;
  -webkit-app-region: drag; 
}

.mascot-container {
  position: relative;
  width: 100px;
  height: 100px;
  transition: transform 0.2s;
}

.mascot-container:hover {
  transform: scale(1.1);
}

.mascot-icon {
  width: 100%;
  height: 100%;
  border-radius: 50%;
  border: 3px solid #fff;
  box-shadow: 0 4px 15px rgba(255, 105, 180, 0.5);
  object-fit: cover;
  z-index: 2;
  position: relative;
}

.pulse-ring {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  border-radius: 50%;
  border: 2px solid rgba(255, 105, 180, 0.6);
  animation: pulse 2s infinite;
  z-index: 1;
}

@keyframes pulse {
  0% { transform: scale(1); opacity: 0.8; }
  100% { transform: scale(1.5); opacity: 0; }
}

/* Overlay */
.galgame-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  background: rgba(0, 0, 0, 0.3);
  z-index: 9999;
}

.character-art {
  position: absolute;
  bottom: 0;
  right: 5%;
  height: 85vh;
  object-fit: contain;
  filter: drop-shadow(0 0 20px rgba(255, 255, 255, 0.2));
  animation: slideIn 0.5s ease-out;
  z-index: 1;
}

.dialogue-container {
  position: absolute;
  bottom: 20px;
  left: 50%;
  transform: translateX(-50%);
  width: 90%;
  max-width: 800px;
  height: 200px;
  z-index: 2;
}

.dialogue-bg {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  opacity: 0.9;
  border-radius: 15px;
  box-shadow: 0 10px 30px rgba(0,0,0,0.5);
}

.dialogue-content {
  position: relative;
  z-index: 3;
  padding: 25px 40px;
  color: #fff;
  height: 100%;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 10px;
}

.name-tag {
  background: linear-gradient(90deg, #ff6b6b, #ff8e53);
  padding: 5px 15px;
  border-radius: 20px;
  font-weight: bold;
  font-size: 1.1rem;
  box-shadow: 0 2px 10px rgba(255, 107, 107, 0.4);
}

.close-btn {
  background: none;
  border: none;
  color: rgba(255, 255, 255, 0.7);
  font-size: 2rem;
  cursor: pointer;
  line-height: 1;
}

.message-area {
  flex-grow: 1;
  font-size: 1.2rem;
  line-height: 1.6;
  text-shadow: 0 2px 4px rgba(0,0,0,0.5);
}

.typing-effect {
  color: #a8e6cf;
}

.cursor {
  animation: blink 1s infinite;
}

.error-msg {
  color: #ff6b6b;
  font-weight: bold;
}

.retry-btn {
  margin-left: 10px;
  padding: 5px 15px;
  background: #fff;
  color: #ff6b6b;
  border: none;
  border-radius: 5px;
  cursor: pointer;
  font-weight: bold;
}

/* Options */
.options-wrapper {
  position: absolute;
  top: 40%;
  left: 20%; /* Position to the left of character */
  transform: translateY(-50%);
  display: flex;
  flex-direction: column;
  gap: 15px;
  z-index: 10;
  width: 400px;
}

.option-btn {
  background: rgba(255, 255, 255, 0.9);
  backdrop-filter: blur(5px);
  padding: 15px 20px;
  border-radius: 10px;
  cursor: pointer;
  transition: all 0.2s;
  border-left: 5px solid #ccc;
  box-shadow: 0 5px 15px rgba(0,0,0,0.2);
  display: flex;
  align-items: center;
  gap: 10px;
  animation: popIn 0.3s cubic-bezier(0.175, 0.885, 0.32, 1.275) backwards;
}

.option-btn:hover {
  transform: translateX(10px) scale(1.02);
  background: #fff;
}

.option-btn.tsundere { border-color: #ff6b6b; animation-delay: 0.1s; }
.option-btn.sweet { border-color: #48dbfb; animation-delay: 0.2s; }
.option-btn.funny { border-color: #feca57; animation-delay: 0.3s; }

.tag {
  font-size: 0.8rem;
  padding: 2px 8px;
  border-radius: 4px;
  color: #fff;
  font-weight: bold;
  white-space: nowrap;
}

.tsundere .tag { background: #ff6b6b; }
.sweet .tag { background: #48dbfb; }
.funny .tag { background: #feca57; }

.text {
  color: #333;
  font-weight: 500;
}

@keyframes blink { 50% { opacity: 0; } }
@keyframes slideIn { from { transform: translateX(100px); opacity: 0; } to { transform: translateX(0); opacity: 1; } }
@keyframes popIn { from { transform: scale(0.8); opacity: 0; } to { transform: scale(1); opacity: 1; } }
</style>