<script setup lang="ts">
import { ref, onMounted } from "vue";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { GoogleGenerativeAI } from "@google/generative-ai";

const API_KEY = "AIzaSyCsMdS9v6totSlUtTlPZ4BR9v0BFrgLbLk";
const genAI = new GoogleGenerativeAI(API_KEY);

const options = ref<{ tsundere: string; sweet: string; funny: string } | null>(null);
const loading = ref(false);
const error = ref("");
const showOverlay = ref(false);

// Start analysis when the floating widget is clicked
const startAnalysis = async () => {
  loading.value = true;
  error.value = "";
  options.value = null;
  // This will trigger the backend to hide window -> capture -> show window (fullscreen)
  await invoke("analyze");
};

// Close overlay and return to widget mode
const closeOverlay = async () => {
  showOverlay.value = false;
  options.value = null;
  await invoke("reset_window");
};

onMounted(async () => {
  // Listen for the image from backend
  await listen<string>("analyze-chat", async (event) => {
    showOverlay.value = true;
    loading.value = true;
    
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
            data: event.payload,
            mimeType: "image/png",
          },
        },
      ]);

      const response = await result.response;
      const text = response.text();
      const jsonStr = text.replace(/```json/g, "").replace(/```/g, "").trim();
      options.value = JSON.parse(jsonStr);
    } catch (e) {
      console.error(e);
      error.value = "分析失败，请重试";
    } finally {
      loading.value = false;
    }
  });

  await listen("analyze-error", (event) => {
    console.error(event.payload);
    error.value = "无法捕获窗口";
    showOverlay.value = true; // Show overlay to display error
  });
});

const copyToClipboard = async (text: string) => {
  try {
    await navigator.clipboard.writeText(text);
    alert("已复制: " + text);
    closeOverlay();
  } catch (err) {
    console.error("Failed to copy: ", err);
  }
};
</script>

<template>
  <!-- Floating Widget Mode -->
  <div v-if="!showOverlay" class="floating-widget" @click="startAnalysis" data-tauri-drag-region>
    <div class="soul-core">
      <div class="core-inner"></div>
      <div class="core-ring"></div>
    </div>
  </div>

  <!-- Fullscreen Overlay Mode -->
  <div v-else class="galgame-overlay">
    <div class="glass-panel">
      <div class="header">
        <span class="title">Galgame Chat Assistant</span>
        <button class="close-btn" @click="closeOverlay">×</button>
      </div>
      
      <div v-if="loading" class="loading-state">
        <div class="spinner"></div>
        <p>正在分析当前对话...</p>
      </div>

      <div v-else-if="error" class="error-state">
        {{ error }}
        <button @click="closeOverlay" class="retry-btn">关闭</button>
      </div>

      <div v-else-if="options" class="options-container">
        <div class="option-card tsundere" @click="copyToClipboard(options.tsundere)">
          <div class="label">傲娇 (Tsundere)</div>
          <div class="text">{{ options.tsundere }}</div>
        </div>
        <div class="option-card sweet" @click="copyToClipboard(options.sweet)">
          <div class="label">温柔 (Sweet)</div>
          <div class="text">{{ options.sweet }}</div>
        </div>
        <div class="option-card funny" @click="copyToClipboard(options.funny)">
          <div class="label">搞笑 (Funny)</div>
          <div class="text">{{ options.funny }}</div>
        </div>
      </div>
    </div>
  </div>
</template>

<style>
/* Global reset for transparent window */
html, body, #app {
  margin: 0;
  padding: 0;
  width: 100%;
  height: 100%;
  background: transparent;
  overflow: hidden;
  user-select: none;
}
</style>

<style scoped>
/* Floating Widget Styles */
.floating-widget {
  width: 100vw;
  height: 100vh;
  display: flex;
  justify-content: center;
  align-items: center;
  cursor: pointer;
  /* Make sure the widget is draggable */
  -webkit-app-region: drag; 
}

.soul-core {
  position: relative;
  width: 80px;
  height: 80px;
  display: flex;
  justify-content: center;
  align-items: center;
}

.core-inner {
  width: 50px;
  height: 50px;
  background: radial-gradient(circle, #ff6b6b, #ff4757);
  border-radius: 50%;
  box-shadow: 0 0 20px #ff6b6b;
  animation: breathe 3s infinite ease-in-out;
  z-index: 2;
}

.core-ring {
  position: absolute;
  width: 70px;
  height: 70px;
  border: 2px solid rgba(255, 107, 107, 0.5);
  border-radius: 50%;
  animation: spin 10s linear infinite;
}

@keyframes breathe {
  0%, 100% { transform: scale(0.95); box-shadow: 0 0 15px #ff6b6b; }
  50% { transform: scale(1.05); box-shadow: 0 0 30px #ff6b6b; }
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* Overlay Styles */
.galgame-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  background: rgba(0, 0, 0, 0.4); /* Darken background slightly */
  display: flex;
  justify-content: center;
  align-items: center;
  z-index: 9999;
  backdrop-filter: blur(4px);
}

.glass-panel {
  background: rgba(20, 20, 35, 0.9);
  backdrop-filter: blur(16px);
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 20px;
  padding: 2rem;
  width: 90%;
  max-width: 600px;
  box-shadow: 0 8px 32px 0 rgba(31, 38, 135, 0.37);
  color: white;
  animation: fadeIn 0.3s ease-out;
  pointer-events: auto; /* Ensure panel is clickable */
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 2rem;
}

.title {
  font-size: 1.2rem;
  font-weight: bold;
  background: linear-gradient(45deg, #ff6b6b, #feca57);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
}

.close-btn {
  background: none;
  border: none;
  color: rgba(255, 255, 255, 0.6);
  font-size: 1.5rem;
  cursor: pointer;
}

.loading-state {
  text-align: center;
  padding: 2rem;
}

.spinner {
  width: 40px;
  height: 40px;
  border: 4px solid rgba(255, 255, 255, 0.1);
  border-left-color: #ff6b6b;
  border-radius: 50%;
  animation: spin 1s linear infinite;
  margin: 0 auto 1rem;
}

.options-container {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.option-card {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.05);
  border-radius: 12px;
  padding: 1rem;
  cursor: pointer;
  transition: all 0.2s;
  position: relative;
  overflow: hidden;
}

.option-card:hover {
  transform: translateY(-2px);
  background: rgba(255, 255, 255, 0.1);
}

.option-card::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  width: 4px;
  height: 100%;
}

.tsundere::before { background: #ff6b6b; }
.sweet::before { background: #48dbfb; }
.funny::before { background: #feca57; }

.label {
  font-size: 0.8rem;
  text-transform: uppercase;
  letter-spacing: 1px;
  margin-bottom: 0.5rem;
  opacity: 0.8;
}

.text {
  font-size: 1.1rem;
  line-height: 1.4;
}

.retry-btn {
  margin-top: 1rem;
  padding: 0.5rem 1rem;
  background: #ff6b6b;
  border: none;
  border-radius: 4px;
  color: white;
  cursor: pointer;
}

@keyframes fadeIn {
  from { opacity: 0; transform: translateY(10px); }
  to { opacity: 1; transform: translateY(0); }
}
</style>