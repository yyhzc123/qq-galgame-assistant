```
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
const showDialogue = ref(false);
const currentEventPayload = ref<string | null>(null);
const isSilentMode = ref(false);
const isAnalyzingSilent = ref(false);
const hasSilentResult = ref(false);

// Start analysis
const startAnalysis = async () => {
  if (hasSilentResult.value) {
    await invoke("analyze", { silent: false });
    return;
  }

  loading.value = true;
  error.value = "";
  options.value = null;
  currentEventPayload.value = null;
  isAnalyzingSilent.value = isSilentMode.value;
  
  await invoke("analyze", { silent: isSilentMode.value });
};

// Close dialogue
const closeDialogue = async () => {
  showDialogue.value = false;
  options.value = null;
  hasSilentResult.value = false;
  error.value = "";
  await invoke("reset_window");
};

const quitApp = async () => {
  await invoke("quit");
};

const analyzeImage = async (base64Image: string) => {
  if (hasSilentResult.value && showDialogue.value) {
     loading.value = false;
     return;
  }

  loading.value = true;
  error.value = "";
  
  try {
    const model = genAI.getGenerativeModel({ model: "gemini-2.5-flash" });
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
    
    if (isAnalyzingSilent.value) {
      hasSilentResult.value = true;
      isAnalyzingSilent.value = false;
      loading.value = false;
    }
  } catch (e: any) {
    console.error("Analysis Error:", e);
    error.value = "Error: " + (e.message || JSON.stringify(e));
    isAnalyzingSilent.value = false;
  } finally {
    if (!isAnalyzingSilent.value) {
        loading.value = false;
    }
  }
};

onMounted(async () => {
  await listen<string>("analyze-chat", async (event) => {
    if (!isAnalyzingSilent.value) {
        showDialogue.value = true;
    }
    currentEventPayload.value = event.payload;
    await analyzeImage(event.payload);
  });

  await listen("trigger-analyze", async () => {
    await startAnalysis();
  });

  await listen("analyze-error", (event) => {
    console.error(event.payload);
    error.value = "无法捕获窗口";
    if (!isAnalyzingSilent.value) showDialogue.value = true;
  });
});

const copyToClipboard = async (text: string) => {
  try {
    await navigator.clipboard.writeText(text);
    closeDialogue();
  } catch (err) {
    console.error("Failed to copy: ", err);
  }
};

const retryAnalysis = () => {
  hasSilentResult.value = false;
  if (currentEventPayload.value) {
    analyzeImage(currentEventPayload.value);
  } else {
    closeDialogue();
    startAnalysis();
  }
};

const toggleSilentMode = (e: Event) => {
    e.stopPropagation();
    isSilentMode.value = !isSilentMode.value;
}
</script>

<template>
  <!-- Widget Mode: Cute Badge Style -->
  <div v-if="!showDialogue" class="widget-container" data-tauri-drag-region>
    <div class="cute-badge" @click="startAnalysis" :class="{ 'analyzing': loading && isAnalyzingSilent, 'ready': hasSilentResult }">
        <div class="paw-print">
            <div class="pad main"></div>
            <div class="pad toe t1"></div>
            <div class="pad toe t2"></div>
            <div class="pad toe t3"></div>
        </div>
        <div class="badge-label">
            <span v-if="hasSilentResult">查看</span>
            <span v-else>Start</span>
        </div>
    </div>

    <!-- Mini Controls -->
    <div class="mini-controls">
        <button class="mini-btn" @click="toggleSilentMode" :class="{ 'active': isSilentMode }" title="静默模式">
            <div class="indicator" :class="{ 'on': isSilentMode }"></div>
            <span>Silent</span>
        </button>
        <button class="mini-btn quit" @click="quitApp" title="退出">
            <span>Exit</span>
        </button>
    </div>
  </div>

  <!-- Dialogue Mode: Visual Novel Menu Style -->
  <div v-else class="vn-container">
      <div class="vn-box">
          <!-- Decorative Header -->
          <div class="vn-header" data-tauri-drag-region>
              <div class="header-ribbon">
                  <span class="header-text">Galgame Assistant</span>
              </div>
              <button class="vn-close-btn" @click="closeDialogue">Close</button>
          </div>

          <div class="vn-content">
            <!-- Loading State -->
            <div v-if="loading" class="vn-loading">
                <div class="bounce-dot"></div>
                <div class="bounce-dot"></div>
                <div class="bounce-dot"></div>
            </div>
            
            <!-- Error State -->
            <div v-else-if="error" class="vn-error">
                <p>{{ error }}</p>
                <button class="vn-retry-btn" @click="retryAnalysis">Retry</button>
            </div>

            <!-- Options List -->
            <div v-else-if="options" class="vn-options">
                <div class="vn-option-card pink" @click="copyToClipboard(options.tsundere)">
                    <div class="card-label">Tsundere</div>
                    <div class="card-text">{{ options.tsundere }}</div>
                </div>
                <div class="vn-option-card blue" @click="copyToClipboard(options.sweet)">
                    <div class="card-label">Sweet</div>
                    <div class="card-text">{{ options.sweet }}</div>
                </div>
                <div class="vn-option-card yellow" @click="copyToClipboard(options.funny)">
                    <div class="card-label">Funny</div>
                    <div class="card-text">{{ options.funny }}</div>
                </div>
            </div>
          </div>
      </div>
  </div>
</template>

<style>
@import url('https://fonts.googleapis.com/css2?family=M+PLUS+Rounded+1c:wght@400;700&display=swap');

html, body, #app {
  margin: 0;
  padding: 0;
  width: 100%;
  height: 100%;
  background: transparent;
  overflow: hidden;
  user-select: none;
  font-family: 'M PLUS Rounded 1c', sans-serif; /* Cute rounded font */
}
</style>

<style scoped>
/* --- Widget Styles --- */
.widget-container {
  width: 100vw;
  height: 100vh;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  gap: 8px;
}

.cute-badge {
    width: 65px;
    height: 65px;
    background: #fff5e6; /* Cream */
    border: 3px solid #ffb7b2; /* Pastel Pink */
    border-radius: 50%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    cursor: pointer;
    box-shadow: 0 4px 10px rgba(142, 110, 83, 0.2); /* Soft brown shadow */
    transition: all 0.2s cubic-bezier(0.175, 0.885, 0.32, 1.275);
    position: relative;
}

.cute-badge:hover {
    transform: scale(1.1) rotate(5deg);
    border-color: #ff9e99;
}

.cute-badge.analyzing {
    border-color: #ffd93d; /* Yellow */
    animation: wiggle 1s infinite;
}

.cute-badge.ready {
    border-color: #6ff7c1; /* Mint */
    background: #e0fff4;
}

/* Paw Print CSS */
.paw-print {
    position: relative;
    width: 30px;
    height: 30px;
    margin-bottom: 2px;
}

.pad {
    background: #ffb7b2;
    position: absolute;
    border-radius: 50%;
}

.cute-badge.ready .pad { background: #6ff7c1; }

.pad.main {
    width: 16px; height: 14px;
    bottom: 2px; left: 7px;
    border-radius: 40% 40% 50% 50%;
}

.pad.toe { width: 8px; height: 10px; top: 0; }
.t1 { left: 0; transform: rotate(-20deg); }
.t2 { left: 11px; top: -2px; }
.t3 { right: 0; transform: rotate(20deg); }

.badge-label {
    font-size: 0.7rem;
    color: #8e6e53; /* Chocolate */
    font-weight: bold;
}

.mini-controls {
    display: flex;
    gap: 8px;
    background: rgba(255, 255, 255, 0.9);
    padding: 4px 8px;
    border-radius: 12px;
    box-shadow: 0 2px 5px rgba(0,0,0,0.1);
}

.mini-btn {
    background: none;
    border: none;
    font-size: 0.7rem;
    color: #8e6e53;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 2px 4px;
    border-radius: 4px;
    transition: background 0.2s;
}

.mini-btn:hover { background: #fff0f5; }

.indicator {
    width: 6px; height: 6px;
    border-radius: 50%;
    background: #ddd;
}
.indicator.on { background: #ffb7b2; }

/* --- Visual Novel Menu Styles --- */
.vn-container {
    width: 100vw;
    height: 100vh;
    display: flex;
    justify-content: center;
    align-items: center;
    /* Polka dot pattern background */
    background-color: transparent; 
}

.vn-box {
    width: 95%;
    max-width: 750px;
    background: #fffbf5; /* Light Cream */
    border: 4px solid #fff;
    outline: 2px dashed #ffb7b2; /* Dashed pink outline */
    border-radius: 15px;
    box-shadow: 0 10px 25px rgba(142, 110, 83, 0.15);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    position: relative;
}

/* Decorative corner patterns */
.vn-box::after {
    content: '';
    position: absolute;
    top: 0; left: 0; right: 0; bottom: 0;
    background-image: radial-gradient(#ffb7b2 1px, transparent 1px);
    background-size: 20px 20px;
    opacity: 0.1;
    pointer-events: none;
}

.vn-header {
    height: 40px;
    background: #fff0f5; /* Lavender Blush */
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 15px;
    border-bottom: 2px solid #fff;
    position: relative;
    z-index: 2;
    cursor: grab;
    -webkit-app-region: drag;
}

.vn-header:active { cursor: grabbing; }

.header-ribbon {
    background: #ffb7b2;
    padding: 2px 15px;
    border-radius: 15px;
    color: #fff;
    font-size: 0.9rem;
    font-weight: bold;
    box-shadow: 0 2px 0 rgba(0,0,0,0.1);
}

.vn-close-btn {
    background: #fff;
    border: 1px solid #ffb7b2;
    color: #ffb7b2;
    border-radius: 10px;
    padding: 2px 10px;
    font-size: 0.8rem;
    cursor: pointer;
    font-weight: bold;
    transition: all 0.2s;
}

.vn-close-btn:hover {
    background: #ffb7b2;
    color: #fff;
}

.vn-content {
    padding: 20px;
    min-height: 220px;
    display: flex;
    justify-content: center;
    align-items: center;
    position: relative;
    z-index: 2;
}

/* Loading Animation */
.vn-loading {
    display: flex;
    gap: 8px;
}

.bounce-dot {
    width: 12px; height: 12px;
    background: #ffb7b2;
    border-radius: 50%;
    animation: bounce 0.6s infinite alternate;
}
.bounce-dot:nth-child(2) { animation-delay: 0.2s; background: #a2d2ff; }
.bounce-dot:nth-child(3) { animation-delay: 0.4s; background: #ffd93d; }

@keyframes bounce { to { transform: translateY(-10px); } }
@keyframes wiggle { 0%, 100% { transform: rotate(-5deg); } 50% { transform: rotate(5deg); } }

.vn-error {
    color: #ff6b6b;
    text-align: center;
}

.vn-retry-btn {
    margin-top: 10px;
    background: #ff6b6b;
    color: #fff;
    border: none;
    padding: 5px 15px;
    border-radius: 15px;
    cursor: pointer;
}

/* Options */
.vn-options {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 12px;
}

.vn-option-card {
    background: #fff;
    border: 2px solid #eee;
    border-radius: 12px;
    padding: 12px 15px;
    cursor: pointer;
    transition: all 0.2s;
    position: relative;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    gap: 4px;
}

.vn-option-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 5px 15px rgba(0,0,0,0.05);
}

.vn-option-card.pink { border-left: 5px solid #ffb7b2; }
.vn-option-card.pink:hover { border-color: #ffb7b2; background: #fff0f5; }
.vn-option-card.pink .card-label { color: #ff8e88; }

.vn-option-card.blue { border-left: 5px solid #a2d2ff; }
.vn-option-card.blue:hover { border-color: #a2d2ff; background: #f0f8ff; }
.vn-option-card.blue .card-label { color: #74b9ff; }

.vn-option-card.yellow { border-left: 5px solid #ffd93d; }
.vn-option-card.yellow:hover { border-color: #ffd93d; background: #fffdf0; }
.vn-option-card.yellow .card-label { color: #f4c724; }

.card-label {
    font-size: 0.75rem;
    font-weight: bold;
    text-transform: uppercase;
    letter-spacing: 1px;
}

.card-text {
    color: #555;
    font-size: 1rem;
    line-height: 1.4;
}
</style>
```