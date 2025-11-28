const loading = ref(false);
const error = ref("");
const showDialogue = ref(false);
const currentEventPayload = ref<string | null>(null);

// Start analysis when the floating widget is clicked
const startAnalysis = async () => {
  loading.value = true;
  error.value = "";
  options.value = null;
  currentEventPayload.value = null;
  await invoke("analyze");
};

// Close dialogue and return to widget mode
const closeDialogue = async () => {
  showDialogue.value = false;
  options.value = null;
  error.value = "";
  await invoke("reset_window");
};

const analyzeImage = async (base64Image: string) => {
  loading.value = true;
  error.value = "";
  
  try {
    // Use the requested Gemini 2.5 Flash model
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
  } catch (e: any) {
    console.error("Analysis Error:", e);
    error.value = "Error: " + (e.message || JSON.stringify(e));
  } finally {
    loading.value = false;
  }
};

onMounted(async () => {
  await listen<string>("analyze-chat", async (event) => {
    showDialogue.value = true;
    currentEventPayload.value = event.payload;
    await analyzeImage(event.payload);
  });

  // Handle global shortcut trigger
  await listen("trigger-analyze", async () => {
    await startAnalysis();
  });

  await listen("analyze-error", (event) => {
    console.error(event.payload);
    error.value = "无法捕获窗口: " + JSON.stringify(event.payload);
    showDialogue.value = true;
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
  if (currentEventPayload.value) {
    analyzeImage(currentEventPayload.value);
  } else {
    closeDialogue();
    startAnalysis();
  }
};
</script>

<template>
  <!-- Widget Mode -->
  <div v-if="!showDialogue" class="widget-container" @click="startAnalysis" data-tauri-drag-region>
    <div class="system-core">
      <div class="core-center"></div>
      <div class="core-orbit"></div>
    </div>
  </div>

  <!-- Dialogue Mode (Fills the resized window) -->
  <div v-else class="dialogue-container">
    <div class="dialogue-header">
      <span class="system-label">SYSTEM ANALYSIS</span>
      <button class="close-btn" @click="closeDialogue">×</button>
    </div>

    <div class="content-area">
      <div v-if="loading" class="status-msg typing">
        正在解析当前对话场景...
      </div>
      
      <div v-else-if="error" class="status-msg error">
        {{ error }} <span class="retry-link" @click="retryAnalysis">[重试]</span>
      </div>

      <div v-else-if="options" class="options-grid">
        <div class="option-btn" @click="copyToClipboard(options.tsundere)">
          <span class="tag tsundere">傲娇</span>
          <span class="text">{{ options.tsundere }}</span>
        </div>
        <div class="option-btn" @click="copyToClipboard(options.sweet)">
          <span class="tag sweet">温柔</span>
          <span class="text">{{ options.sweet }}</span>
        </div>
        <div class="option-btn" @click="copyToClipboard(options.funny)">
          <span class="tag funny">搞笑</span>
          <span class="text">{{ options.funny }}</span>
        </div>
      </div>
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
/* Widget Styles */
.widget-container {
  width: 100vw;
  height: 100vh;
  display: flex;
  justify-content: center;
  align-items: center;
  cursor: pointer;
  /* Ensure drag region works */
}

.system-core {
  position: relative;
  width: 60px;
  height: 60px;
  display: flex;
  justify-content: center;
  align-items: center;
  transition: transform 0.2s;
}

.system-core:hover {
  transform: scale(1.1);
}

.core-center {
  width: 30px;
  height: 30px;
  background: #00f2ff;
  border-radius: 50%;
  box-shadow: 0 0 10px #00f2ff;
  animation: pulse 3s infinite ease-in-out;
}

.core-orbit {
  position: absolute;
  width: 100%;
  height: 100%;
  border: 2px solid rgba(0, 242, 255, 0.3);
  border-radius: 50%;
  border-left-color: #00f2ff;
  animation: spin 2s linear infinite;
}

/* Dialogue Styles */
.dialogue-container {
  width: 100vw;
  height: 100vh;
  background: rgba(15, 20, 30, 0.95);
  border: 1px solid rgba(0, 242, 255, 0.2);
  display: flex;
  flex-direction: column;
  box-sizing: border-box;
  padding: 15px;
  color: #fff;
  border-radius: 8px;
  box-shadow: 0 0 20px rgba(0,0,0,0.5);
}

.dialogue-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 15px;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  padding-bottom: 10px;
}

.system-label {
  font-size: 0.8rem;
  letter-spacing: 2px;
  color: #00f2ff;
  font-weight: bold;
}

.close-btn {
  background: none;
  border: none;
  color: rgba(255, 255, 255, 0.5);
  font-size: 1.2rem;
  cursor: pointer;
}

.close-btn:hover {
  color: #fff;
}

.content-area {
  flex-grow: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}

.status-msg {
  font-size: 1.1rem;
  color: #a8e6cf;
}

.status-msg.error {
  color: #ff6b6b;
}

.retry-link {
  text-decoration: underline;
  cursor: pointer;
  margin-left: 10px;
  color: #fff;
}

.options-grid {
  display: grid;
  grid-template-columns: 1fr 1fr 1fr;
  gap: 15px;
  width: 100%;
}

.option-btn {
  background: rgba(255, 255, 255, 0.05);
  border: 1px solid rgba(255, 255, 255, 0.1);
  padding: 15px;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.option-btn:hover {
  background: rgba(255, 255, 255, 0.1);
  border-color: #00f2ff;
  transform: translateY(-2px);
}

.tag {
  font-size: 0.75rem;
  padding: 2px 6px;
  border-radius: 3px;
  align-self: flex-start;
  font-weight: bold;
}

.tag.tsundere { background: #ff6b6b; color: #fff; }
.tag.sweet { background: #48dbfb; color: #000; }
.tag.funny { background: #feca57; color: #000; }

.text {
  font-size: 0.95rem;
  line-height: 1.4;
  color: #eee;
}

@keyframes pulse { 0%, 100% { opacity: 0.6; transform: scale(0.9); } 50% { opacity: 1; transform: scale(1.1); } }
@keyframes spin { to { transform: rotate(360deg); } }
</style>