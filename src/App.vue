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
  <!-- Floating Widget Mode: Abstract Tech Orb -->
  <div v-if="!showOverlay" class="floating-widget" @click="startAnalysis" data-tauri-drag-region>
    <div class="tech-orb">
      <div class="orb-core"></div>
      <div class="orb-ring"></div>
    </div>
  </div>

  <!-- Fullscreen Overlay Mode -->
  <div v-else class="galgame-overlay">
    
    <!-- Central Choice Menu -->
    <div v-if="options" class="choice-menu">
      <div class="choice-btn" @click="copyToClipboard(options.tsundere)">
        <span class="choice-text">{{ options.tsundere }}</span>
      </div>
      <div class="choice-btn" @click="copyToClipboard(options.sweet)">
        <span class="choice-text">{{ options.sweet }}</span>
      </div>
      <div class="choice-btn" @click="copyToClipboard(options.funny)">
        <span class="choice-text">{{ options.funny }}</span>
      </div>
    </div>

    <!-- Bottom Dialogue Box (System Status) -->
    <div class="dialogue-box">
      <div class="name-box">SYSTEM</div>
      <div class="text-box">
        <div v-if="loading" class="typing">
          正在解析当前对话场景...
        </div>
        <div v-else-if="error" class="error">
          {{ error }} <span class="retry-link" @click="retryAnalysis">[重试]</span>
        </div>
        <div v-else class="instruction">
          已生成回复方案，请做出选择。
        </div>
      </div>
      <button class="close-btn" @click="closeOverlay">×</button>
    </div>
  </div>
</template>

<style>
@import url('https://fonts.googleapis.com/css2?family=Noto+Sans+SC:wght@400;500;700&display=swap');

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

.tech-orb {
  position: relative;
  width: 60px;
  height: 60px;
  display: flex;
  justify-content: center;
  align-items: center;
  transition: transform 0.2s;
}

.tech-orb:hover {
  transform: scale(1.1);
}

.orb-core {
  width: 40px;
  height: 40px;
  background: #00f2ff;
  border-radius: 50%;
  box-shadow: 0 0 15px #00f2ff;
  opacity: 0.8;
  animation: pulse 2s infinite ease-in-out;
}

.orb-ring {
  position: absolute;
  width: 100%;
  height: 100%;
  border: 2px solid rgba(0, 242, 255, 0.5);
  border-radius: 50%;
  border-top-color: transparent;
  animation: spin 4s linear infinite;
}

@keyframes pulse {
  0%, 100% { transform: scale(0.9); opacity: 0.6; }
  50% { transform: scale(1.1); opacity: 0.9; }
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

/* Overlay */
.galgame-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  background: rgba(0, 0, 0, 0.2);
  z-index: 9999;
}

/* Choice Menu - Centered */
.choice-menu {
  position: absolute;
  top: 45%;
  left: 50%;
  transform: translate(-50%, -50%);
  display: flex;
  flex-direction: column;
  gap: 20px;
  width: 80%;
  max-width: 600px;
  z-index: 10;
}

.choice-btn {
  background: rgba(0, 20, 40, 0.85);
  border: 1px solid rgba(0, 242, 255, 0.3);
  padding: 20px 30px;
  text-align: center;
  cursor: pointer;
  transition: all 0.2s;
  position: relative;
  overflow: hidden;
  backdrop-filter: blur(5px);
  box-shadow: 0 4px 10px rgba(0,0,0,0.3);
}

.choice-btn:hover {
  background: rgba(0, 40, 80, 0.95);
  border-color: #00f2ff;
  transform: scale(1.02);
  box-shadow: 0 0 15px rgba(0, 242, 255, 0.4);
}

.choice-btn::before {
  content: '';
  position: absolute;
  left: 0;
  top: 0;
  height: 100%;
  width: 4px;
  background: #00f2ff;
  opacity: 0.5;
}

.choice-text {
  color: #fff;
  font-size: 1.1rem;
  letter-spacing: 1px;
}

/* Dialogue Box - Bottom */
.dialogue-box {
  position: absolute;
  bottom: 50px;
  left: 50%;
  transform: translateX(-50%);
  width: 90%;
  max-width: 900px;
  height: 150px;
  background: rgba(10, 15, 25, 0.9);
  border: 2px solid rgba(255, 255, 255, 0.1);
  border-radius: 4px;
  box-shadow: 0 10px 30px rgba(0,0,0,0.5);
  display: flex;
  flex-direction: column;
  padding: 20px;
  box-sizing: border-box;
  backdrop-filter: blur(10px);
}

.name-box {
  position: absolute;
  top: -20px;
  left: 20px;
  background: #00f2ff;
  color: #000;
  padding: 5px 20px;
  font-weight: bold;
  font-size: 0.9rem;
  transform: skewX(-15deg);
  box-shadow: 2px 2px 0 rgba(0,0,0,0.5);
}

.text-box {
  flex-grow: 1;
  color: #eee;
  font-size: 1.1rem;
  line-height: 1.6;
  padding-top: 10px;
}

.typing {
  color: #00f2ff;
  animation: blink 1.5s infinite;
}

.error {
  color: #ff4757;
}

.retry-link {
  text-decoration: underline;
  cursor: pointer;
  margin-left: 10px;
  color: #fff;
}

.close-btn {
  position: absolute;
  top: 10px;
  right: 15px;
  background: none;
  border: none;
  color: rgba(255, 255, 255, 0.5);
  font-size: 1.5rem;
  cursor: pointer;
  transition: color 0.2s;
}

.close-btn:hover {
  color: #fff;
}

@keyframes blink { 50% { opacity: 0.5; } }
</style>