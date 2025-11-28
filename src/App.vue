```
<script setup lang="ts">
import { ref, onMounted } from "vue";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { GoogleGenerativeAI } from "@google/generative-ai";

const API_KEY = "AIzaSyCsMdS9v6totSlUtTlPZ4BR9v0BFrgLbLk";
const genAI = new GoogleGenerativeAI(API_KEY);

// Dynamic options type
const options = ref<Array<{ style: string; text: string }> | null>(null);
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
    // Improved Prompt for Dynamic Styles
    const prompt = `Analyze the chat history in this image. The last message is the one to reply to. 
    Provide 3 distinct reply options in a Galgame style (in Chinese).
    Instead of fixed archetypes, dynamically choose 3 most suitable styles based on the conversation context (e.g., 傲娇, 温柔, 腹黑, 幽默, 害羞, 调皮, etc.).
    
    Output ONLY a valid JSON array of objects, where each object has:
    - "style": The style name in Chinese (max 4 chars).
    - "text": The reply text.
    
    Example JSON format:
    [
      {"style": "傲娇", "text": "哼，谁...谁稀罕你回消息啊！"},
      {"style": "温柔", "text": "今天也很辛苦呢，要注意休息哦。"},
      {"style": "腹黑", "text": "呵呵，看来你很懂嘛~"}
    ]
    Do not include markdown formatting.`;

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

// Helper to cycle card colors
const getCardClass = (index: number) => {
    const classes = ['pink', 'blue', 'yellow'];
    return classes[index % classes.length];
}
</script>

<template>
  <!-- Widget Mode: Frosted Glass Style -->
  <div v-if="!showDialogue" class="widget-container">
    <div class="widget-glass" data-tauri-drag-region>
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
  </div>

  <!-- Dialogue Mode: Clean Visual Novel Style -->
  <div v-else class="vn-container">
      <div class="vn-box">
          <!-- Header (Draggable) -->
          <div class="vn-header" data-tauri-drag-region>
              <div class="header-ribbon" data-tauri-drag-region>
                  <span class="header-text" data-tauri-drag-region>Galgame Assistant</span>
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

            <!-- Options List (Dynamic) -->
            <div v-else-if="options" class="vn-options">
                <div 
                    v-for="(opt, index) in options" 
                    :key="index"
                    class="vn-option-card" 
                    :class="getCardClass(index)"
                    @click="copyToClipboard(opt.text)"
                >
                    <div class="card-label">{{ opt.style }}</div>
                    <div class="card-text">{{ opt.text }}</div>
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
  font-family: 'M PLUS Rounded 1c', sans-serif;
}
</style>

<style scoped>
/* --- Widget Styles --- */
.widget-container {
  width: 100vw;
  height: 100vh;
  display: flex;
  justify-content: center;
  align-items: center;
}

.widget-glass {
    background: rgba(255, 255, 255, 0.6);
    backdrop-filter: blur(12px);
    -webkit-backdrop-filter: blur(12px);
    padding: 15px;
    border-radius: 25px;
    border: 1px solid rgba(255, 255, 255, 0.8);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.1);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    cursor: grab; /* Indicates draggable */
}

.widget-glass:active {
    cursor: grabbing;
}

.cute-badge {
    width: 65px;
    height: 65px;
    background: #fff5e6;
    border: 3px solid #ffb7b2;
    border-radius: 50%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    cursor: pointer;
    box-shadow: 0 4px 10px rgba(142, 110, 83, 0.1);
    transition: all 0.2s cubic-bezier(0.175, 0.885, 0.32, 1.275);
    position: relative;
    /* Prevent drag on the button itself so click works reliably */
    /* Actually in Tauri, click passes through to drag region if not handled. 
       But we have @click handler. */
}

.cute-badge:hover {
    transform: scale(1.1) rotate(5deg);
    border-color: #ff9e99;
}

.cute-badge.analyzing {
    border-color: #ffd93d;
    animation: wiggle 1s infinite;
}

.cute-badge.ready {
    border-color: #6ff7c1;
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
    color: #8e6e53;
    font-weight: bold;
}

.mini-controls {
    display: flex;
    gap: 8px;
    /* Removed background since parent has glass effect */
}

.mini-btn {
    background: rgba(255,255,255,0.5);
    border: 1px solid rgba(255,255,255,0.8);
    font-size: 0.7rem;
    color: #8e6e53;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    border-radius: 8px;
    transition: all 0.2s;
}

.mini-btn:hover { background: #fff; transform: translateY(-1px); }

.indicator {
    width: 6px; height: 6px;
    border-radius: 50%;
    background: #ccc;
}
.indicator.on { background: #ffb7b2; }

/* --- Visual Novel Menu Styles --- */
.vn-container {
    width: 100vw;
    height: 100vh;
    display: flex;
    justify-content: center;
    align-items: center;
    background-color: transparent; 
}

.vn-box {
    width: 95%;
    max-width: 750px;
    background: rgba(255, 251, 245, 0.95); /* Slightly transparent cream */
    backdrop-filter: blur(10px);
    border-radius: 15px;
    box-shadow: 0 10px 30px rgba(0,0,0,0.1);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    position: relative;
    /* Removed the dashed outline and thick border */
    border: 1px solid rgba(255, 255, 255, 0.8);
}

/* Decorative corner patterns */
/* Removed .vn-box::after */

.vn-header {
    height: 45px;
    background: linear-gradient(to right, #fff0f5, #fff5e6);
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 15px;
    border-bottom: 1px solid rgba(0,0,0,0.05);
    cursor: grab;
    -webkit-app-region: drag;
}

.vn-header:active { cursor: grabbing; }

.header-ribbon {
    background: #ffb7b2;
    padding: 4px 12px;
    border-radius: 12px;
    color: #fff;
    font-size: 0.85rem;
    font-weight: bold;
    box-shadow: 0 2px 5px rgba(255, 183, 178, 0.4);
}

.vn-close-btn {
    background: transparent;
    border: 1px solid #ffb7b2;
    color: #ffb7b2;
    border-radius: 8px;
    padding: 4px 12px;
    font-size: 0.75rem;
    cursor: pointer;
    font-weight: bold;
    transition: all 0.2s;
}

.vn-close-btn:hover {
    background: #ffb7b2;
    color: #fff;
}

.vn-content {
    padding: 25px;
    min-height: 200px;
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
    padding: 20px;
}

.vn-retry-btn {
    margin-top: 10px;
    background: #ff6b6b;
    color: #fff;
    border: none;
    padding: 6px 16px;
    border-radius: 20px;
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
    border: 1px solid #f0f0f0;
    border-radius: 12px;
    padding: 15px 20px;
    cursor: pointer;
    transition: all 0.2s;
    position: relative;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    gap: 6px;
    box-shadow: 0 2px 5px rgba(0,0,0,0.02);
}

.vn-option-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 5px 15px rgba(0,0,0,0.08);
}

.vn-option-card.pink { border-left: 4px solid #ffb7b2; }
.vn-option-card.pink:hover { border-color: #ffb7b2; background: #fff5f7; }
.vn-option-card.pink .card-label { color: #ff8e88; }

.vn-option-card.blue { border-left: 4px solid #a2d2ff; }
.vn-option-card.blue:hover { border-color: #a2d2ff; background: #f0f8ff; }
.vn-option-card.blue .card-label { color: #74b9ff; }

.vn-option-card.yellow { border-left: 4px solid #ffd93d; }
.vn-option-card.yellow:hover { border-color: #ffd93d; background: #fffdf5; }
.vn-option-card.yellow .card-label { color: #f4c724; }

.card-label {
    font-size: 0.75rem;
    font-weight: bold;
    letter-spacing: 1px;
}

.card-text {
    color: #555;
    font-size: 1rem;
    line-height: 1.5;
}
</style>
```